public class J {
  public static final String S01 = "\b\s\t\n\f\r\"\'\\";
  public static final String S02 = "\0|\7|\77|\377|\400|\1234";
  public static final String S03 = "\u005cn";
  public static final String S04 = "\u005c\u005c";
  public static final String S05 = "a\u005c\u0022b";
  public static final String S06 = "\uuuu0041\u0042";
  public static final String S07 = "\u005c\u0074";
  public static final String S08 = "\\u0041";
  public static final String S09 = "\\\u005c\u005c";
  public static final String S10 = "x\u005c\u0030y";
  public static final String S11 = "\uD83D\uDE00|\uD83D|\uDE00";
  public static final char C01 = '\u005c\u0027';
  public static final char C02 = '\u0041';
  public static final char C03 = '\377';
  public static final char C04 = '\u005cn';
  public static final char C05 = '\\';
  public static final char C06 = '\uD83D';
  public static final String T01 = """
      a\tb\u005cn
        c\"\"\" end\
       joined \s
      """;
}
