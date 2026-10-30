package org.mobileofficefree.app

import java.util.Base64
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class InvoiceTest {
    private val xslt = """<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:n1="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
  xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <xsl:output method="html" encoding="UTF-8"/>
  <xsl:template match="/"><html><body><h1>SATICI TASARIMI</h1><p><xsl:value-of select="n1:Invoice/cbc:ID"/></p></body></html></xsl:template>
</xsl:stylesheet>"""

    private fun invoice(withXslt: Boolean, profile: String = "TICARIFATURA"): ByteArray {
        val attachment = if (withXslt) """
  <cac:AdditionalDocumentReference>
    <cbc:ID>1</cbc:ID><cbc:IssueDate>2026-10-30</cbc:IssueDate><cbc:DocumentType>XSLT</cbc:DocumentType>
    <cac:Attachment><cbc:EmbeddedDocumentBinaryObject mimeCode="application/xml" encodingCode="Base64" characterSetCode="UTF-8" filename="GIB.xslt">${Base64.getEncoder().encodeToString(xslt.toByteArray())}</cbc:EmbeddedDocumentBinaryObject></cac:Attachment>
  </cac:AdditionalDocumentReference>""" else ""
        return """<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
  xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
  xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:UBLVersionID>2.1</cbc:UBLVersionID>
  <cbc:ProfileID>$profile</cbc:ProfileID>
  <cbc:ID>ABC2026000000123</cbc:ID>
  <cbc:UUID>3f2c1b7e-1111-4a2b-9c3d-123456789abc</cbc:UUID>
  <cbc:IssueDate>2026-10-30</cbc:IssueDate>
  <cbc:IssueTime>14:05:00</cbc:IssueTime>
  <cbc:InvoiceTypeCode>SATIS</cbc:InvoiceTypeCode>
  <cbc:Note>Yalnız bin iki yüz TL'dir.</cbc:Note>
  <cbc:DocumentCurrencyCode>TRY</cbc:DocumentCurrencyCode>$attachment
  <cac:AccountingSupplierParty><cac:Party>
    <cac:PartyIdentification><cbc:ID schemeID="VKN">1234567890</cbc:ID></cac:PartyIdentification>
    <cac:PartyName><cbc:Name>Örnek Yazılım Ltd. Şti.</cbc:Name></cac:PartyName>
    <cac:PostalAddress><cbc:StreetName>İstiklal Cad.</cbc:StreetName><cbc:BuildingNumber>12</cbc:BuildingNumber><cbc:CitySubdivisionName>Beyoğlu</cbc:CitySubdivisionName><cbc:CityName>İstanbul</cbc:CityName><cac:Country><cbc:Name>Türkiye</cbc:Name></cac:Country></cac:PostalAddress>
    <cac:PartyTaxScheme><cac:TaxScheme><cbc:Name>Beyoğlu</cbc:Name></cac:TaxScheme></cac:PartyTaxScheme>
  </cac:Party></cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty><cac:Party>
    <cac:PartyIdentification><cbc:ID schemeID="TCKN">11111111111</cbc:ID></cac:PartyIdentification>
    <cac:Person><cbc:FirstName>Ayşe</cbc:FirstName><cbc:FamilyName>Yılmaz</cbc:FamilyName></cac:Person>
  </cac:Party></cac:AccountingCustomerParty>
  <cac:TaxTotal><cbc:TaxAmount currencyID="TRY">200.00</cbc:TaxAmount>
    <cac:TaxSubtotal><cbc:TaxableAmount currencyID="TRY">1000.00</cbc:TaxableAmount><cbc:TaxAmount currencyID="TRY">200.00</cbc:TaxAmount><cbc:Percent>20</cbc:Percent></cac:TaxSubtotal>
  </cac:TaxTotal>
  <cac:LegalMonetaryTotal>
    <cbc:LineExtensionAmount currencyID="TRY">1000.00</cbc:LineExtensionAmount>
    <cbc:TaxExclusiveAmount currencyID="TRY">1000.00</cbc:TaxExclusiveAmount>
    <cbc:TaxInclusiveAmount currencyID="TRY">1200.00</cbc:TaxInclusiveAmount>
    <cbc:PayableAmount currencyID="TRY">1200.00</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>
  <cac:InvoiceLine>
    <cbc:ID>1</cbc:ID><cbc:InvoicedQuantity unitCode="C62">2</cbc:InvoicedQuantity>
    <cbc:LineExtensionAmount currencyID="TRY">1000.00</cbc:LineExtensionAmount>
    <cac:TaxTotal><cbc:TaxAmount currencyID="TRY">200.00</cbc:TaxAmount><cac:TaxSubtotal><cbc:Percent>20</cbc:Percent></cac:TaxSubtotal></cac:TaxTotal>
    <cac:Item><cbc:Name>Yazılım lisansı (yıllık)</cbc:Name></cac:Item>
    <cac:Price><cbc:PriceAmount currencyID="TRY">500.00</cbc:PriceAmount></cac:Price>
  </cac:InvoiceLine>
</Invoice>""".toByteArray()
    }

    @Test
    fun readsTheSummary() {
        val s = Invoices.read(invoice(withXslt = false))
        assertEquals(InvoiceKind.INVOICE, s.kind)
        assertEquals("ABC2026000000123", s.number)
        assertEquals("3f2c1b7e-1111-4a2b-9c3d-123456789abc", s.uuid)
        assertEquals("2026-10-30 14:05", s.date)
        assertEquals("Örnek Yazılım Ltd. Şti.", s.supplier.name)
        assertEquals("1234567890", s.supplier.taxId)
        assertEquals("VKN", s.supplier.taxIdScheme)
        assertEquals("Beyoğlu", s.supplier.taxOffice)
        assertTrue(s.supplier.address.contains("İstanbul"))
        assertEquals("Ayşe Yılmaz", s.customer.name)
        assertEquals("TCKN", s.customer.taxIdScheme)
        assertEquals(1, s.lines.size)
        assertEquals("Yazılım lisansı (yıllık)", s.lines[0].name)
        assertEquals("Adet", Invoices.unitName(s.lines[0].unit))
        assertEquals("1.200,00 TL", s.payable)
        assertEquals("200,00 TL", s.taxTotal)
        assertNull(s.xslt)
        assertEquals("e-Fatura", Invoices.title(s))
        assertEquals("e-Arşiv Fatura", Invoices.title(Invoices.read(invoice(false, "EARSIVFATURA"))))
    }

    @Test
    fun usesTheSellersEmbeddedDesignWhenThereIsOne() {
        val bytes = invoice(withXslt = true)
        val s = Invoices.read(bytes)
        assertTrue(s.xslt != null)
        val (html, own) = Invoices.html(bytes, s)
        assertTrue(own)
        assertTrue(html.contains("SATICI TASARIMI") && html.contains("ABC2026000000123"))
    }

    @Test
    fun fallsBackToThePlainLayout() {
        val bytes = invoice(withXslt = false)
        val (html, own) = Invoices.html(bytes, Invoices.read(bytes))
        assertFalse(own)
        assertTrue(html.contains("Örnek Yazılım Ltd. Şti.") && html.contains("1.200,00 TL") && html.contains("Hesaplanan KDV"))
    }

    @Test
    fun recognisesUblAndRefusesExternalEntities() {
        assertTrue(Invoices.looksLikeUbl(String(invoice(false)).take(2000)))
        assertFalse(Invoices.looksLikeUbl("<note>hello</note>"))
        val evil = """<?xml version="1.0"?><!DOCTYPE x [<!ENTITY e SYSTEM "file:///etc/passwd">]><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2">&e;</Invoice>"""
        val result = runCatching { Invoices.read(evil.toByteArray()) }
        // DTDs are refused: reading fails rather than fetching the file.
        assertTrue(result.isFailure)
    }
}
