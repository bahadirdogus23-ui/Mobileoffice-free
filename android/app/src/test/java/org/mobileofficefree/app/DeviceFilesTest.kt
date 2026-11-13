package org.mobileofficefree.app

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class DeviceFilesTest {
    @get:Rule val folder = TemporaryFolder()

    @Test
    fun includesRemovableStorageWithoutRepeatingPrimaryOrInventingRoots() {
        val primary = File(folder.root, "internal")
        val card = File(folder.root, "card")
        assertEquals(listOf(primary, card), storageRoots(primary, listOf(
            File(primary, "Android/data/org.mobileofficefree.app/files"),
            File(card, "Android/data/org.mobileofficefree.app/files"),
            null,
            File(folder.root, "unexpected/cache/files"),
        )))
    }

    @Test
    fun findsUdfInDownloadsAndSharedMediaButSkipsPrivateAndHiddenFolders() {
        val paths = listOf(
            "Download/Dilekçe.UDF", "Android/media/chat/Documents/Ek.udf",
            "Android/data/chat/private.udf", "Android/obb/chat/private.udf", ".hidden/secret.udf",
        )
        paths.forEach { path -> File(folder.root, path).apply { parentFile.mkdirs(); writeText("document") } }
        File(folder.root, "Download/folder.udf").mkdir()
        assertEquals(setOf("Dilekçe.UDF", "Ek.udf"), DeviceFiles.findDocuments(folder.root, setOf("udf")) { true }.map { it.name }.toSet())
    }

    @Test
    fun subsequentWalkFindsNewlyDownloadedUdf() {
        assertEquals(emptyList<File>(), DeviceFiles.findDocuments(folder.root, setOf("udf")) { true })
        val downloaded = folder.newFile("Yeni.udf")
        assertEquals(listOf(downloaded), DeviceFiles.findDocuments(folder.root, setOf("udf")) { true })
    }
}
