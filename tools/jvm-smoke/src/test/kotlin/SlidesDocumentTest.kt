import kotlin.test.Test
import kotlin.test.assertFailsWith
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.SlidesDocument

class SlidesDocumentTest {
    @Test
    fun notAPresentationIsAnOpenError() {
        assertFailsWith<MoException.Open> { SlidesDocument.open("sunum değil".toByteArray()) }
    }
}
