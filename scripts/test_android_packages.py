import pathlib
import struct
import tempfile
import unittest
import zipfile

from check_android_packages import inspect_package


def elf(machine=183, alignment=16384):
    data = bytearray(120)
    data[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<H", data, 18, machine)
    struct.pack_into("<Q", data, 0x20, 64)
    struct.pack_into("<HH", data, 0x36, 56, 1)
    struct.pack_into("<I", data, 64, 1)
    struct.pack_into("<Q", data, 64 + 0x30, alignment)
    return data


class PackageChecks(unittest.TestCase):
    def package(self, entries, bundle=False):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        path = pathlib.Path(temp.name) / ("test.aab" if bundle else "test.apk")
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("BundleConfig.pb" if bundle else "AndroidManifest.xml", b"fixture")
            for name, data in entries.items():
                archive.writestr(name, data)
        return path

    def test_arm64_apk_and_size_accounting(self):
        path = self.package({"lib/arm64-v8a/libmo_ffi.so": elf(), "assets/ocr": b"model"})
        report = inspect_package(path, ["arm64-v8a"])
        self.assertEqual(report["bytes"], path.stat().st_size)
        self.assertEqual(report["native_libraries"][0]["bytes"], 120)

    def test_bundle_layout(self):
        path = self.package({"base/lib/arm64-v8a/libmo_ffi.so": elf()}, bundle=True)
        self.assertEqual(inspect_package(path, ["arm64-v8a"])["abis"], ["arm64-v8a"])

    def test_missing_engine_despite_other_native_library(self):
        path = self.package({"lib/arm64-v8a/libjna.so": elf()})
        with self.assertRaisesRegex(ValueError, "missing native document engine"):
            inspect_package(path, ["arm64-v8a"])

    def test_missing_abi(self):
        path = self.package({"lib/arm64-v8a/libmo_ffi.so": elf()})
        with self.assertRaisesRegex(ValueError, "expected ABIs"):
            inspect_package(path, ["arm64-v8a", "x86_64"])

    def test_wrong_machine(self):
        path = self.package({"lib/arm64-v8a/libmo_ffi.so": elf(machine=62)})
        with self.assertRaisesRegex(ValueError, "wrong ELF architecture"):
            inspect_package(path, ["arm64-v8a"])

    def test_dependency_alignment_is_checked_too(self):
        path = self.package({"lib/arm64-v8a/libmo_ffi.so": elf(), "lib/arm64-v8a/libocr.so": elf(alignment=4096)})
        with self.assertRaisesRegex(ValueError, "invalid LOAD alignment"):
            inspect_package(path, ["arm64-v8a"])

    def test_32_bit_arm_is_exempt_from_16kb_alignment(self):
        data = bytearray(84)
        data[:6] = b"\x7fELF\x01\x01"
        struct.pack_into("<H", data, 18, 40)
        struct.pack_into("<I", data, 0x1C, 52)
        struct.pack_into("<HH", data, 0x2A, 32, 1)
        struct.pack_into("<I", data, 52, 1)
        struct.pack_into("<I", data, 52 + 0x1C, 4096)
        path = self.package({"lib/armeabi-v7a/libmo_ffi.so": data})
        self.assertEqual(inspect_package(path, ["armeabi-v7a"])["abis"], ["armeabi-v7a"])

    def test_wrong_elf_class(self):
        path = self.package({"lib/armeabi-v7a/libmo_ffi.so": elf(machine=40)})
        with self.assertRaisesRegex(ValueError, "wrong ELF class"):
            inspect_package(path, ["armeabi-v7a"])

    def test_invalid_native_header(self):
        path = self.package({"lib/arm64-v8a/libmo_ffi.so": b"not a native library"})
        with self.assertRaisesRegex(ValueError, "invalid ELF header"):
            inspect_package(path, ["arm64-v8a"])


if __name__ == "__main__":
    unittest.main()
