// The Java side of tests/classpath/jvm/classfile_reader_lone_surrogate, compiled by javac into lonesurrogate-lib
// (tests/support/jars.sh) and put on the class path: a constant whose class file entry holds a lone surrogate
// (modified UTF-8).
public class J {
    public static final String S = "\uD83D";
    public static final String T = "a\uDE00b";
}
