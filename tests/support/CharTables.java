// The tables of java.lang.Character that the interpreter and the JavaScript runtime answer
// `isDigit`, `isLetter`, `isLetterOrDigit`, `digit`, `asDigit` and `getNumericValue` of a `Char`
// by, and the fix tool's parser its `getType` and `isLowerCase` (scalameta's `Chars`), over all
// 65,536 UTF-16 units: `src/interp/char_tables.rs` whole, and the two table lines of
// `runtime/rt.js`. Run from the repository's root on JDK 24, the JDK the JVM target's references
// run on: `java tests/support/CharTables.java`; a diff after it is a stale table.
// The tables are data of the Unicode Character Database, version 16.0.0 in JDK 24, so the Rust
// file begins with Unicode's license as the JDK carries it (legal/java.base/unicode.md), which
// NOTICE carries too.
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class CharTables {
    static final String[] UNICODE_LICENSE = {
        "UNICODE LICENSE V3",
        "",
        "COPYRIGHT AND PERMISSION NOTICE",
        "",
        "Copyright © 1991-2024 Unicode, Inc.",
        "",
        "NOTICE TO USER: Carefully read the following legal agreement. BY",
        "DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR",
        "SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE",
        "TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT",
        "DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.",
        "",
        "Permission is hereby granted, free of charge, to any person obtaining a",
        "copy of data files and any associated documentation (the \"Data Files\") or",
        "software and any associated documentation (the \"Software\") to deal in the",
        "Data Files or Software without restriction, including without limitation",
        "the rights to use, copy, modify, merge, publish, distribute, and/or sell",
        "copies of the Data Files or Software, and to permit persons to whom the",
        "Data Files or Software are furnished to do so, provided that either (a)",
        "this copyright and permission notice appear with all copies of the Data",
        "Files or Software, or (b) this copyright and permission notice appear in",
        "associated Documentation.",
        "",
        "THE DATA FILES AND SOFTWARE ARE PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY",
        "KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF",
        "MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF",
        "THIRD PARTY RIGHTS.",
        "",
        "IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE",
        "BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,",
        "OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,",
        "WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,",
        "ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA",
        "FILES OR SOFTWARE.",
        "",
        "Except as contained in this notice, the name of a copyright holder shall",
        "not be used in advertising or otherwise to promote the sale, use or other",
        "dealings in these Data Files or Software without prior written",
        "authorization of the copyright holder.",
    };

    public static void main(String[] args) throws Exception {
        String version = System.getProperty("java.specification.version");
        if (!version.equals("24")) {
            System.err.println("the tables are JDK 24's; this is JDK " + version);
            System.exit(1);
        }
        List<Integer> zeros = new ArrayList<>();
        List<int[]> runs = new ArrayList<>();
        List<int[]> letters = new ArrayList<>();
        List<int[]> types = runsOf(u -> Character.getType((char) u));
        List<int[]> lower = runsOf(u -> Character.isLowerCase((char) u) ? 1 : 0);
        for (int c = 0; c < 65536; c++) {
            char ch = (char) c;
            if (Character.isDigit(ch) && Character.digit(ch, 10) == 0) {
                zeros.add(c);
            }
            if (Character.isLetter(ch)) {
                if (!letters.isEmpty() && letters.get(letters.size() - 1)[1] == c - 1) {
                    letters.get(letters.size() - 1)[1] = c;
                } else {
                    letters.add(new int[] {c, c});
                }
            }
        }
        // The other numeric values in runs whose value is constant or grows by one.
        int c = 0;
        while (c < 65536) {
            int v = Character.getNumericValue((char) c);
            if (v == -1 || Character.isDigit((char) c)) {
                c++;
                continue;
            }
            int next = c + 1 < 65536 && !Character.isDigit((char) (c + 1)) ? Character.getNumericValue((char) (c + 1)) : -1;
            int step = v >= 0 && next == v + 1 ? 1 : 0;
            int end = c;
            while (end + 1 < 65536 && !Character.isDigit((char) (end + 1))
                    && Character.getNumericValue((char) (end + 1)) == v + (end + 1 - c) * step) {
                end++;
            }
            runs.add(new int[] {c, end, v, step});
            c = end + 1;
        }
        check(zeros, runs, letters);
        checkRuns(types, u -> Character.getType((char) u));
        checkRuns(lower, u -> Character.isLowerCase((char) u) ? 1 : 0);

        StringBuilder rs = new StringBuilder();
        rs.append("//! Generated by tests/support/CharTables.java from JDK 24's `java.lang.Character`; do not\n");
        rs.append("//! edit. The interpreter's `isDigit`, `isLetter`, `digit` and `getNumericValue` of a `Char`, and\n");
        rs.append("//! the fix tool's parser's `getType` and `isLowerCase` (scalameta's `Chars`).\n\n");
        rs.append("// Data of the Unicode Character Database 16.0.0, as JDK 24 carries it, under its license:\n//\n");
        for (String line : UNICODE_LICENSE) {
            rs.append(line.isEmpty() ? "//\n" : "// " + line + "\n");
        }
        rs.append("\n");
        rs.append("/// The zero of every run of ten decimal digits.\n");
        rs.append("pub const DIGIT_ZEROS: [u16; ").append(zeros.size()).append("] = [");
        for (int i = 0; i < zeros.size(); i++) {
            rs.append(i % 12 == 0 ? "\n    " : " ").append(String.format("0x%04x,", zeros.get(i)));
        }
        rs.append("\n];\n\n");
        rs.append("/// Every run of the other numeric values: its first and last unit, the first one's value, and\n");
        rs.append("/// whether the value grows by one along the run. A fraction's value is -2.\n");
        rs.append("pub const NUMERIC_RUNS: [(u16, u16, i32, bool); ").append(runs.size()).append("] = [\n");
        for (int[] r : runs) {
            rs.append(String.format("    (0x%04x, 0x%04x, %d, %b),\n", r[0], r[1], r[2], r[3] == 1));
        }
        rs.append("];\n\n");
        rs.append("/// The runs of letters, first and last unit.\n");
        rs.append("pub const LETTERS: [(u16, u16); ").append(letters.size()).append("] = [");
        for (int i = 0; i < letters.size(); i++) {
            int[] r = letters.get(i);
            rs.append(i % 6 == 0 ? "\n    " : " ").append(String.format("(0x%04x, 0x%04x),", r[0], r[1]));
        }
        rs.append("\n];\n\n");
        rs.append("/// Every run of units of one general category (`Character.getType`): its first unit and the category.\n");
        rs.append("pub const TYPES: [(u16, u8); ").append(types.size()).append("] = [");
        for (int i = 0; i < types.size(); i++) {
            int[] r = types.get(i);
            rs.append(i % 8 == 0 ? "\n    " : " ").append(String.format("(0x%04x, %d),", r[0], r[2]));
        }
        rs.append("\n];\n\n");
        appendRanges(rs, "The runs of lower-case units (`Character.isLowerCase`), first and last unit.", "LOWER_CASE", lower);
        Files.writeString(Path.of("src/interp/char_tables.rs"), rs.toString());

        StringBuilder zeroLine = new StringBuilder("const $digitZeros = \"");
        for (int z : zeros) {
            zeroLine.append(String.format("\\u%04x", z));
        }
        zeroLine.append("\";");
        StringBuilder runLine = new StringBuilder("const $numericRuns = [");
        for (int i = 0; i < runs.size(); i++) {
            int[] r = runs.get(i);
            runLine.append(i == 0 ? "" : ", ").append(r[0]).append(", ").append(r[1] - r[0] + 1).append(", ").append(r[2]).append(", ").append(r[3]);
        }
        runLine.append("];");
        Path rt = Path.of("runtime/rt.js");
        List<String> lines = new ArrayList<>(Files.readAllLines(rt));
        int replaced = 0;
        for (int i = 0; i < lines.size(); i++) {
            if (lines.get(i).startsWith("const $digitZeros = ")) {
                lines.set(i, zeroLine.toString());
                replaced++;
            } else if (lines.get(i).startsWith("const $numericRuns = ")) {
                lines.set(i, runLine.toString());
                replaced++;
            }
        }
        if (replaced != 2) {
            System.err.println("runtime/rt.js has no `const $digitZeros` or `const $numericRuns` line to replace");
            System.exit(1);
        }
        Files.writeString(rt, String.join("\n", lines) + "\n");
        System.out.println(zeros.size() + " digit runs, " + runs.size() + " numeric runs, " + letters.size() + " letter runs, "
            + types.size() + " category runs, " + lower.size() + " lower-case runs");
    }

    // The runs of units over which `f` stays the same: each its first and last unit and the value.
    static List<int[]> runsOf(java.util.function.IntUnaryOperator f) {
        List<int[]> out = new ArrayList<>();
        for (int c = 0; c < 65536; c++) {
            int v = f.applyAsInt(c);
            if (!out.isEmpty() && out.get(out.size() - 1)[2] == v) {
                out.get(out.size() - 1)[1] = c;
            } else {
                out.add(new int[] {c, c, v});
            }
        }
        return out;
    }

    // The runs answer as `f` does, for every unit.
    static void checkRuns(List<int[]> runs, java.util.function.IntUnaryOperator f) {
        for (int[] r : runs) {
            for (int c = r[0]; c <= r[1]; c++) {
                if (f.applyAsInt(c) != r[2]) {
                    throw new AssertionError("run " + r[0] + " unit " + c);
                }
            }
        }
    }

    // The runs of value 1, first and last unit.
    static void appendRanges(StringBuilder rs, String doc, String name, List<int[]> runs) {
        List<int[]> on = new ArrayList<>();
        for (int[] r : runs) {
            if (r[2] == 1) {
                on.add(r);
            }
        }
        rs.append("/// ").append(doc).append("\n");
        rs.append("pub const ").append(name).append(": [(u16, u16); ").append(on.size()).append("] = [");
        for (int i = 0; i < on.size(); i++) {
            int[] r = on.get(i);
            rs.append(i % 6 == 0 ? "\n    " : " ").append(String.format("(0x%04x, 0x%04x),", r[0], r[1]));
        }
        rs.append("\n];\n");
    }

    // The tables answer as Character does, for every unit and every radix.
    static void check(List<Integer> zeros, List<int[]> runs, List<int[]> letters) {
        for (int c = 0; c < 65536; c++) {
            char ch = (char) c;
            int decimal = -1;
            for (int z : zeros) {
                if (c - z >= 0 && c - z < 10) {
                    decimal = c - z;
                }
            }
            int numeric = decimal;
            for (int[] r : runs) {
                if (c >= r[0] && c <= r[1]) {
                    numeric = r[2] + (c - r[0]) * r[3];
                }
            }
            boolean letter = false;
            for (int[] r : letters) {
                letter |= c >= r[0] && c <= r[1];
            }
            int latin = c >= 'A' && c <= 'Z' ? c - 'A' + 10 : c >= 'a' && c <= 'z' ? c - 'a' + 10
                : c >= 0xff21 && c <= 0xff3a ? c - 0xff21 + 10 : c >= 0xff41 && c <= 0xff5a ? c - 0xff41 + 10 : -1;
            int value = decimal >= 0 ? decimal : latin;
            for (int radix = -1; radix <= 37; radix++) {
                int digit = radix >= 2 && radix <= 36 && value < radix ? value : -1;
                if (digit != Character.digit(ch, radix)) {
                    throw new AssertionError("digit " + c + " " + radix);
                }
            }
            if ((decimal >= 0) != Character.isDigit(ch) || numeric != Character.getNumericValue(ch)
                    || letter != Character.isLetter(ch) || (letter || decimal >= 0) != Character.isLetterOrDigit(ch)) {
                throw new AssertionError("unit " + c);
            }
        }
    }
}
