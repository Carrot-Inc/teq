// The oracle of the compiler's text of a Double and a Float (src/javanum.rs, Java's `Double.toString` and
// `Float.toString`, which the typer, the interpreter, the JVM target's analysis and the NIR printer all write
// through): tests/support/java-numbers.txt, written from what this JDK prints for every power of two with its two
// neighbours (the subnormal ones too), 100,000 bit patterns of each width drawn by splitmix64, and the values that
// lie exactly between their two shortest decimals. A set's line holds its count and the SHA-256 of its lines,
// each `<the bits in hex> <the text>`; the ties stand one to a line. Run from the repository's root on JDK 19 or
// later (whose algorithm the JDK specifies; the file is JDK 24's): `java tests/support/JavaNumbers.java`, and a diff
// after it is a stale file; `java tests/support/JavaNumbers.java --lines <set>` prints a set's lines.
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;

public class JavaNumbers {
    static final String[] SETS = {"float-powers", "double-powers", "float-random", "double-random"};

    // Values exactly between the two shortest decimals that read back as them: the JDK prints the even one.
    static final long[] FLOAT_TIES = {
        0x39800000L, 0x4a000001L, 0x48af8064L, 0x4a3660b5L, 0x4a4e1121L, 0xc8c853a4L, 0x4a7ef4e5L, 0x49b6973aL,
        0x46fa12a0L, 0x4880f254L, 0xc9c6bd82L, 0xc9a8dacaL, 0x48e67dc4L, 0x4a234bfdL, 0x49f23112L, 0xca236671L,
        0x48215a08L, 0xca5e3f01L, 0x49fe8dc2L, 0x4a535c55L, 0xca1cc72dL, 0xc9d0a21aL, 0xc98a6ecaL, 0x4a079c41L,
        0xc9847822L, 0xca01065dL, 0xc8d8a664L, 0xca5d18b1L, 0xc8c06454L, 0x48cc7d34L, 0x4a2a3885L, 0xc87efaa8L,
        0x48b738e4L, 0x4a479d2dL, 0x4a4603a5L, 0xc8d73c84L, 0x49bfccc2L, 0xc8837d34L, 0xc8f1a494L,
    };
    static final long[] DOUBLE_TIES = {
        0x3e60000000000000L, 0x4310000000000001L, 0xc2e6aa5728179794L, 0xc30e035117485bb2L,
        0xc31bfa4032e50145L, 0x43015004f433031aL, 0xc317a6c73c6cd981L, 0xc300d851e09c2fd2L,
    };

    static void neighbours(List<Long> out, long bits) {
        out.add(bits - 1);
        out.add(bits);
        out.add(bits + 1);
    }

    static long state = 0;

    // splitmix64 (Steele, Lea and Flood), from 0.
    static long next() {
        state += 0x9e3779b97f4a7c15L;
        long z = state;
        z = (z ^ (z >>> 30)) * 0xbf58476d1ce4e5b9L;
        z = (z ^ (z >>> 27)) * 0x94d049bb133111ebL;
        return z ^ (z >>> 31);
    }

    static List<Long> values(String set) {
        List<Long> out = new ArrayList<>();
        switch (set) {
            case "float-powers" -> {
                for (int k = 0; k < 23; k++) neighbours(out, 1L << k);
                for (long e = 1; e < 255; e++) neighbours(out, e << 23);
            }
            case "double-powers" -> {
                for (int k = 0; k < 52; k++) neighbours(out, 1L << k);
                for (long e = 1; e < 2047; e++) neighbours(out, e << 52);
            }
            case "float-random" -> {
                state = 0;
                for (int i = 0; i < 100_000; i++) out.add(next() >>> 32);
            }
            case "double-random" -> {
                state = 0;
                for (int i = 0; i < 100_000; i++) out.add(next());
            }
            default -> throw new IllegalArgumentException("no set " + set);
        }
        return out;
    }

    static String line(boolean single, long bits) {
        return single
            ? String.format("%08x %s", bits, Float.toString(Float.intBitsToFloat((int) bits)))
            : String.format("%016x %s", bits, Double.toString(Double.longBitsToDouble(bits)));
    }

    static List<String> lines(String set) {
        List<String> out = new ArrayList<>();
        for (long bits : values(set)) out.add(line(set.startsWith("float"), bits));
        return out;
    }

    public static void main(String[] args) throws Exception {
        if (args.length == 2 && args[0].equals("--lines")) {
            for (String l : lines(args[1])) System.out.println(l);
            return;
        }
        StringBuilder file = new StringBuilder();
        file.append("# Float.toString and Double.toString of JDK ").append(Runtime.version().feature())
            .append(", written by tests/support/JavaNumbers.java (its header says how and what).\n");
        for (String set : SETS) {
            MessageDigest sha = MessageDigest.getInstance("SHA-256");
            List<String> ls = lines(set);
            for (String l : ls) sha.update((l + "\n").getBytes(StandardCharsets.UTF_8));
            file.append("set ").append(set).append(' ').append(ls.size()).append(' ')
                .append(HexFormat.of().formatHex(sha.digest())).append('\n');
        }
        for (long bits : FLOAT_TIES) file.append("float ").append(line(true, bits)).append('\n');
        for (long bits : DOUBLE_TIES) file.append("double ").append(line(false, bits)).append('\n');
        Files.writeString(Path.of("tests/support/java-numbers.txt"), file.toString());
        System.out.println("java-numbers: " + SETS.length + " sets, " + (FLOAT_TIES.length + DOUBLE_TIES.length) + " ties");
    }
}
