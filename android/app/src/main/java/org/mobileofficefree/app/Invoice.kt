package org.mobileofficefree.app

import java.io.ByteArrayInputStream
import java.io.StringWriter
import java.util.Base64
import javax.xml.XMLConstants
import javax.xml.parsers.DocumentBuilderFactory
import javax.xml.transform.TransformerFactory
import javax.xml.transform.stream.StreamResult
import javax.xml.transform.stream.StreamSource
import org.w3c.dom.Element
import org.w3c.dom.Node

/** The UBL-TR documents GİB's e-Fatura, e-Arşiv and e-İrsaliye use, by root element. */
enum class InvoiceKind { INVOICE, DESPATCH, CREDIT_NOTE }

/** A party on the document: name, tax number (VKN) or TCKN, tax office and address. */
data class InvoiceParty(val name: String, val taxId: String, val taxIdScheme: String, val taxOffice: String, val address: String)

/** One line: what, how many, unit price, VAT rate and line total. */
data class InvoiceLine(val name: String, val quantity: String, val unit: String, val price: String, val vatPercent: String, val total: String)

/** What the summary card shows, read from the UBL XML. Amounts keep the file's text and currency. */
data class InvoiceSummary(
    val kind: InvoiceKind,
    /** TEMELFATURA, TICARIFATURA, EARSIVFATURA, … */
    val profile: String,
    /** SATIS, IADE, TEVKIFAT, … */
    val type: String,
    val number: String,
    /** The ETTN: GİB's unique id for the document. */
    val uuid: String,
    val date: String,
    val supplier: InvoiceParty,
    val customer: InvoiceParty,
    val lines: List<InvoiceLine>,
    val currency: String,
    val lineTotal: String,
    val taxExclusive: String,
    val taxTotal: String,
    val payable: String,
    val notes: List<String>,
    /** The seller's own XSLT design, embedded in the invoice, if any. */
    val xslt: ByteArray?,
)

/**
 * Reading GİB e-documents (UBL-TR 1.2): the summary, the embedded XSLT if there is one, and an
 * HTML rendering, either through that XSLT or the app's own plain layout. No network: external
 * entities are refused and the HTML is shown with network loads blocked.
 */
object Invoices {
    private const val CBC = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
    private const val CAC = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"

    /** Whether [head] (the first few KB of an XML file) is a UBL document GİB uses. */
    fun looksLikeUbl(head: String): Boolean =
        "urn:oasis:names:specification:ubl:schema:xsd:" in head &&
            listOf("Invoice-2", "DespatchAdvice-2", "CreditNote-2").any { "xsd:$it" in head }

    fun read(bytes: ByteArray): InvoiceSummary {
        val root = parse(bytes).documentElement ?: error("empty XML")
        val kind = when (root.localName) {
            "Invoice" -> InvoiceKind.INVOICE
            "DespatchAdvice" -> InvoiceKind.DESPATCH
            "CreditNote" -> InvoiceKind.CREDIT_NOTE
            else -> error("not a UBL invoice: ${root.localName}")
        }
        val supplierParty = root.child(CAC, if (kind == InvoiceKind.DESPATCH) "DespatchSupplierParty" else "AccountingSupplierParty")
        val customerParty = root.child(CAC, if (kind == InvoiceKind.DESPATCH) "DeliveryCustomerParty" else "AccountingCustomerParty")
        val totals = root.child(CAC, "LegalMonetaryTotal")
        val lineName = when (kind) {
            InvoiceKind.DESPATCH -> "DespatchLine"
            InvoiceKind.CREDIT_NOTE -> "CreditNoteLine"
            InvoiceKind.INVOICE -> "InvoiceLine"
        }
        val qtyName = when (kind) {
            InvoiceKind.DESPATCH -> "DeliveredQuantity"
            InvoiceKind.CREDIT_NOTE -> "CreditedQuantity"
            InvoiceKind.INVOICE -> "InvoicedQuantity"
        }
        return InvoiceSummary(
            kind = kind,
            profile = root.text(CBC, "ProfileID"),
            type = root.text(CBC, if (kind == InvoiceKind.DESPATCH) "DespatchAdviceTypeCode" else "InvoiceTypeCode"),
            number = root.text(CBC, "ID"),
            uuid = root.text(CBC, "UUID"),
            date = listOf(root.text(CBC, "IssueDate"), root.text(CBC, "IssueTime").take(5)).filter { it.isNotEmpty() }.joinToString(" "),
            supplier = party(supplierParty?.child(CAC, "Party")),
            customer = party(customerParty?.child(CAC, "Party")),
            lines = root.children(CAC, lineName).map { line ->
                val qty = line.child(CBC, qtyName)
                InvoiceLine(
                    name = line.child(CAC, "Item")?.text(CBC, "Name").orEmpty(),
                    quantity = qty?.textContent?.trim().orEmpty(),
                    unit = qty?.getAttribute("unitCode").orEmpty(),
                    price = line.child(CAC, "Price")?.amount(CBC, "PriceAmount").orEmpty(),
                    vatPercent = line.child(CAC, "TaxTotal")?.child(CAC, "TaxSubtotal")?.text(CBC, "Percent").orEmpty(),
                    total = line.amount(CBC, "LineExtensionAmount"),
                )
            },
            currency = root.text(CBC, "DocumentCurrencyCode").ifEmpty { "TRY" },
            lineTotal = totals?.amount(CBC, "LineExtensionAmount").orEmpty(),
            taxExclusive = totals?.amount(CBC, "TaxExclusiveAmount").orEmpty(),
            taxTotal = root.children(CAC, "TaxTotal").firstOrNull()?.amount(CBC, "TaxAmount").orEmpty(),
            payable = totals?.amount(CBC, "PayableAmount").orEmpty(),
            notes = root.children(CBC, "Note").map { it.textContent.trim() }.filter { it.isNotEmpty() },
            xslt = embeddedXslt(root),
        )
    }

    /**
     * The design GİB's portal and integrators embed: an AdditionalDocumentReference whose
     * attachment is a base64 file named *.xslt (DocumentType "XSLT" on most).
     */
    private fun embeddedXslt(root: Element): ByteArray? {
        for (ref in root.children(CAC, "AdditionalDocumentReference")) {
            val obj = ref.child(CAC, "Attachment")?.child(CBC, "EmbeddedDocumentBinaryObject") ?: continue
            val name = obj.getAttribute("filename")
            val type = ref.text(CBC, "DocumentType")
            if (!name.endsWith(".xslt", ignoreCase = true) && !name.endsWith(".xsl", ignoreCase = true) && !type.equals("XSLT", ignoreCase = true)) continue
            return runCatching { Base64.getMimeDecoder().decode(obj.textContent.trim()) }.getOrNull()
        }
        return null
    }

    /**
     * The document as HTML: through its own XSLT when it has one that works, else [plainHtml].
     * Returns the HTML and whether the seller's design was used.
     */
    fun html(bytes: ByteArray, summary: InvoiceSummary): Pair<String, Boolean> {
        summary.xslt?.let { xslt ->
            runCatching { transform(bytes, xslt) }.getOrNull()?.takeIf { it.isNotBlank() }?.let { return it to true }
        }
        return plainHtml(summary) to false
    }

    private fun transform(xml: ByteArray, xslt: ByteArray): String {
        val factory = TransformerFactory.newInstance()
        runCatching { factory.setFeature(XMLConstants.FEATURE_SECURE_PROCESSING, true) }
        val transformer = factory.newTransformer(StreamSource(ByteArrayInputStream(xslt)))
        val out = StringWriter()
        transformer.transform(StreamSource(ByteArrayInputStream(xml)), StreamResult(out))
        return out.toString()
    }

    /** The app's own layout: parties, lines and totals in plain tables. */
    fun plainHtml(s: InvoiceSummary): String = buildString {
        fun e(t: String) = t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;")
        fun party(title: String, p: InvoiceParty) {
            append("<td style='vertical-align:top;width:50%'><b>").append(e(title)).append("</b><br>").append(e(p.name))
            if (p.taxId.isNotEmpty()) append("<br>").append(e(p.taxIdScheme.ifEmpty { "VKN" })).append(": ").append(e(p.taxId))
            if (p.taxOffice.isNotEmpty()) append("<br>Vergi Dairesi: ").append(e(p.taxOffice))
            if (p.address.isNotEmpty()) append("<br>").append(e(p.address))
            append("</td>")
        }
        append("<!DOCTYPE html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width'>")
        append("<style>body{font-family:sans-serif;font-size:14px;margin:12px;color:#111}table{border-collapse:collapse;width:100%}")
        append("td,th{border:1px solid #bbb;padding:4px 6px}th{background:#eee;text-align:left}.n{text-align:right;white-space:nowrap}")
        append("h2{margin:4px 0}</style></head><body>")
        append("<h2>").append(e(title(s))).append("</h2>")
        append("<p>No: <b>").append(e(s.number)).append("</b> &nbsp; Tarih: ").append(e(s.date))
        if (s.uuid.isNotEmpty()) append("<br>ETTN: ").append(e(s.uuid))
        if (s.profile.isNotEmpty()) append("<br>Senaryo: ").append(e(s.profile)).append(" &nbsp; Tip: ").append(e(s.type))
        append("</p><table><tr>")
        party(if (s.kind == InvoiceKind.DESPATCH) "Gönderen" else "Satıcı", s.supplier)
        party(if (s.kind == InvoiceKind.DESPATCH) "Alıcı" else "Alıcı", s.customer)
        append("</tr></table><br><table><tr><th>Mal / Hizmet</th><th class='n'>Miktar</th><th class='n'>Birim Fiyat</th><th class='n'>KDV %</th><th class='n'>Tutar</th></tr>")
        for (l in s.lines) {
            append("<tr><td>").append(e(l.name)).append("</td><td class='n'>").append(e(l.quantity)).append(' ').append(e(unitName(l.unit)))
            append("</td><td class='n'>").append(e(l.price)).append("</td><td class='n'>").append(e(l.vatPercent))
            append("</td><td class='n'>").append(e(l.total)).append("</td></tr>")
        }
        append("</table><br><table style='width:auto;margin-left:auto'>")
        for ((label, value) in listOf("Mal Hizmet Toplam" to s.lineTotal, "KDV Matrahı" to s.taxExclusive, "Hesaplanan KDV" to s.taxTotal, "Ödenecek Tutar" to s.payable)) {
            if (value.isNotEmpty()) append("<tr><th>").append(e(label)).append("</th><td class='n'>").append(e(value)).append("</td></tr>")
        }
        append("</table>")
        for (n in s.notes) append("<p>").append(e(n)).append("</p>")
        append("</body></html>")
    }

    fun title(s: InvoiceSummary): String = when {
        s.kind == InvoiceKind.DESPATCH -> "e-İrsaliye"
        s.profile.equals("EARSIVFATURA", ignoreCase = true) -> "e-Arşiv Fatura"
        s.kind == InvoiceKind.CREDIT_NOTE -> "İade Faturası"
        else -> "e-Fatura"
    }

    /** UN/ECE unit codes GİB invoices use most, in Turkish. */
    fun unitName(code: String): String = when (code.uppercase(java.util.Locale.ROOT)) {
        "C62", "NIU" -> "Adet"
        "KGM" -> "Kg"
        "GRM" -> "g"
        "LTR" -> "Lt"
        "MTR" -> "m"
        "MTK" -> "m²"
        "MTQ" -> "m³"
        "HUR" -> "Saat"
        "DAY" -> "Gün"
        "MON" -> "Ay"
        "ANN" -> "Yıl"
        "PA" -> "Paket"
        "BX" -> "Kutu"
        "SET" -> "Set"
        "KWH" -> "kWh"
        else -> code
    }

    private fun party(p: Element?): InvoiceParty {
        if (p == null) return InvoiceParty("", "", "", "", "")
        val ids = p.children(CAC, "PartyIdentification").mapNotNull { it.child(CBC, "ID") }
        val taxId = ids.firstOrNull { it.getAttribute("schemeID") in setOf("VKN", "TCKN") } ?: ids.firstOrNull()
        val person = p.child(CAC, "Person")
        val name = p.child(CAC, "PartyName")?.text(CBC, "Name").orEmpty().ifEmpty {
            listOfNotNull(person?.text(CBC, "FirstName"), person?.text(CBC, "FamilyName")).filter { it.isNotEmpty() }.joinToString(" ")
        }
        val addr = p.child(CAC, "PostalAddress")
        val address = if (addr == null) "" else listOf(
            addr.text(CBC, "StreetName"),
            addr.text(CBC, "BuildingNumber"),
            addr.text(CBC, "CitySubdivisionName"),
            addr.text(CBC, "CityName"),
            addr.child(CAC, "Country")?.text(CBC, "Name").orEmpty(),
        ).filter { it.isNotEmpty() }.joinToString(", ")
        return InvoiceParty(
            name = name,
            taxId = taxId?.textContent?.trim().orEmpty(),
            taxIdScheme = taxId?.getAttribute("schemeID").orEmpty(),
            taxOffice = p.child(CAC, "PartyTaxScheme")?.child(CAC, "TaxScheme")?.text(CBC, "Name").orEmpty(),
            address = address,
        )
    }

    private fun parse(bytes: ByteArray): org.w3c.dom.Document {
        val f = DocumentBuilderFactory.newInstance()
        f.isNamespaceAware = true
        f.isExpandEntityReferences = false
        // Refuse DTDs and external entities (XXE); not every platform parser knows every switch.
        for (feature in listOf(
            "http://apache.org/xml/features/disallow-doctype-decl",
            "http://xml.org/sax/features/external-general-entities",
            "http://xml.org/sax/features/external-parameter-entities",
        )) {
            runCatching { f.setFeature(feature, feature.endsWith("disallow-doctype-decl")) }
        }
        return f.newDocumentBuilder().parse(ByteArrayInputStream(bytes))
    }

    // Small DOM helpers: direct children by namespace and local name.
    private fun Element.children(ns: String, name: String): List<Element> {
        val out = ArrayList<Element>()
        var n: Node? = firstChild
        while (n != null) {
            if (n is Element && n.namespaceURI == ns && n.localName == name) out += n
            n = n.nextSibling
        }
        return out
    }

    private fun Element.child(ns: String, name: String): Element? = children(ns, name).firstOrNull()

    private fun Element.text(ns: String, name: String): String = child(ns, name)?.textContent?.trim().orEmpty()

    /** An amount with its currency, "1.234,50 TRY" (Turkish grouping), or "" when absent. */
    private fun Element.amount(ns: String, name: String): String {
        val el = child(ns, name) ?: return ""
        return formatAmount(el.textContent.trim(), el.getAttribute("currencyID"))
    }

    fun formatAmount(value: String, currency: String): String {
        val n = value.toBigDecimalOrNull() ?: return listOf(value, currency).filter { it.isNotEmpty() }.joinToString(" ")
        val fmt = java.text.DecimalFormat("#,##0.00", java.text.DecimalFormatSymbols(java.util.Locale.forLanguageTag("tr-TR")))
        val text = fmt.format(n.setScale(2, java.math.RoundingMode.HALF_UP))
        val sym = when (currency.uppercase(java.util.Locale.ROOT)) {
            "TRY", "TRL", "" -> "TL"
            else -> currency
        }
        return "$text $sym"
    }
}
