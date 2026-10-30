package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Test
import org.mobileofficefree.engine.FieldOption
import org.mobileofficefree.engine.FormFieldKind
import org.mobileofficefree.engine.PdfFormField

class FormTest {
    private fun field(name: String, kind: FormFieldKind, value: String) =
        PdfFormField(name, name, kind, value, listOf(FieldOption("E", "Evet")), false, false, 0u)

    @Test
    fun onlyRealChangesAreWritten() {
        val fields = listOf(
            field("ad", FormFieldKind.TEXT, "Şule"),
            field("onay", FormFieldKind.CHECK_BOX, ""),
            field("cevap", FormFieldKind.RADIO, "E"),
        )
        val inputs = mapOf(
            "ad" to FieldInput.Text("Şule"),
            "onay" to FieldInput.Check(true),
            "cevap" to FieldInput.Choice("E"),
            "yok" to FieldInput.Text("x"),
        )
        assertEquals(mapOf("onay" to FieldInput.Check(true)), formChanges(fields, inputs))
    }
}
