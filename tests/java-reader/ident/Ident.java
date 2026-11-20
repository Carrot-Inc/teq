// Which characters a Java source's identifier may start with and go on with, as scalac 3.8.4's Java scanner
// (JavaScanners.scala, fetchToken and getIdentRest) reads them, in its order: the ASCII letters, `$` and `_`, else
// Character.isUnicodeIdentifierStart; the ASCII letters and digits, `$` and `_`, else SU (U+001A) ends one, else
// Character.isUnicodeIdentifierPart, of any other character, the identifier-ignorable ASCII controls among them.
// The scanner reads UTF-16 units, so a code point above U+FFFF, a surrogate pair, is neither. Prints the ranges.
public class Ident {
    static boolean start(int cp) {
        if (cp > 0xFFFF) return false;
        char c = (char) cp;
        if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || c == '$' || c == '_') return true;
        return Character.isUnicodeIdentifierStart(c);
    }
    static boolean part(int cp) {
        if (cp > 0xFFFF) return false;
        char c = (char) cp;
        if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '$' || c == '_') return true;
        if (c == '\u001A') return false;
        return Character.isUnicodeIdentifierPart(c);
    }
    static void ranges(String name, java.util.function.IntPredicate p) {
        int lo = -1;
        for (int cp = 0; cp <= 0x110000; cp++) {
            boolean in = cp <= 0x10FFFF && p.test(cp);
            if (in && lo < 0) lo = cp;
            if (!in && lo >= 0) { System.out.println(name + " " + lo + " " + (cp - 1)); lo = -1; }
        }
    }
    public static void main(String[] args) {
        System.out.println("# java " + System.getProperty("java.version") + ", Unicode of its Character");
        ranges("start", Ident::start);
        ranges("part", Ident::part);
    }
}
