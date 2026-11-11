import unittest
from runtime_expected import runtime_expected


class LeadingWarnings(unittest.TestCase):
    warning = b"tests/cases/example.scala:12:3: warning: unchecked type test\n    case x: C => x\n    ^^^^^^^^^^^^^^\n"

    def test_leading_block_only(self):
        output = b"program output\n" + self.warning
        self.assertEqual(runtime_expected(self.warning * 2 + output), output)

    def test_error_stops_the_block(self):
        error = self.warning.replace(b": warning:", b": error:")
        self.assertEqual(runtime_expected(self.warning + error + self.warning), error + self.warning)
        error = error.replace(b"unchecked type test", b"quoted.scala:1:2: warning: message text")
        self.assertEqual(runtime_expected(self.warning + error), error)

    def test_incomplete_or_malformed_warning_is_kept(self):
        for text in [b"warning: plain\nsource\n^\n", *[
            b"".join(self.warning.splitlines(keepends=True)[:n]) for n in [1, 2]
        ], self.warning.replace(b"^^^^^^^^^^^^^^", b"^^ text"),
            self.warning.replace(b":12:3:", b":line:3:")]:
            with self.subTest(text=text):
                self.assertEqual(runtime_expected(self.warning + text), text)

    def test_output_bytes_and_line_endings_are_preserved(self):
        self.assertEqual(runtime_expected(self.warning + b"no final newline"), b"no final newline")
        warning = self.warning.replace(b"\n", b"\r\n")
        self.assertEqual(runtime_expected(warning + b"out\r\n"), b"out\r\n")
        self.assertEqual(runtime_expected(b"\n" + self.warning), b"\n" + self.warning)


if __name__ == "__main__":
    unittest.main()
