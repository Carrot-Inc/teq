package fix;

/** Java varargs of a reference type other than Object: the array a caller spreads into one
 *  arrives as it is, which a write into it shows (tests/cases/java_varargs_spread.scala). */
public class Varargs {
    public static void set(CharSequence... values) {
        values[0] = "changed";
    }

    public static int count(Number... values) {
        return values.length;
    }
}
