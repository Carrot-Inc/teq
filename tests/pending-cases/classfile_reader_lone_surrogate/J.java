// The Java side of tests/pending-cases/classfile_reader_lone_surrogate, compiled by javac 21 and put on the class path
// of both compilers: a constant whose class file entry holds a lone surrogate (modified UTF-8).
public class J {
    public static final String S = "\uD83D";
    public static final String T = "a\uDE00b";
}
