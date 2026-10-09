// Comments mode as JDK 24 reads it: `(?x)` passes over ASCII white space and `#` comments,
// inside classes too; a comment ends at a line terminator, of which NEL, LS and PS stay
// characters of the pattern; the mode is lexical, restored at the end of its group and turned
// off by `(?-x)`; an escaped or quoted character is kept. Where the JDK's parser takes the next
// character as it stands (after a backslash, `(?`, `{`, a class's `^` and a range's hyphen)
// white space is no comment's. `Pattern.COMMENTS` is in tests/classpath/js/regex_runtime_sets.scala.
object Main:
  def shown(s: String): String =
    s.flatMap(c => if c < '!' || c > '~' then "U+" + Integer.toHexString(c).toUpperCase + " " else c.toString).trim
  def check(p: String, inputs: String*): Unit =
    val answer =
      try
        val r = p.r
        inputs.map(s => shown(s) + ":" + r.matches(s)).mkString(" ")
      catch case _: IllegalArgumentException => "rejected"
    println(shown(p) + " = " + answer)
  def main(args: Array[String]): Unit =
    check("(?x)a b c", "abc", "a b c")
    check("(?x)a b c # the rest\n d", "abcd", "abc")
    check("(?x)[a b]", "a", "b", " ")
    check("(?x)[\\Q #\\E]", " ", "#", "a")
    check("(?x)[a#b]\n]", "a", "b", "]", "#")
    check("(?x)[a-c # letters\n x-z]", "b", "y", " ", "#")
    check("(?x)a#note\rb", "ab")
    check("(?x)a#note\u0085b", "a\u0085b", "ab")
    check("(?x)a#note\u2028b", "a\u2028b", "ab")
    check("(?x)a#note\u2029b", "a\u2029b", "ab")
    check("(?x)a\u2028b\u00a0c", "a\u2028b\u00a0c", "abc")
    check("(?x)a\\ b\\#c", "a b#c", "abc")
    check("(?x)a\\Q b # \\Ec", "a b # c", "abc")
    check("a(?x: b ) c", "ab c", "abc")
    check("(?x)a b(?-x) c", "ab c", "abc")
    check("(?x:a b)|c d", "ab", "c d", "cd")
    check("((?x) a b ) c", "ab c", "abc")
    check("(?x) (?i) a b", "AB", "ab")
    check("(?ix)A B", "ab")
    check("(?x)(?x i)a", "A")
    check("(?x)(?i x)a", "A")
    check("(?x)[ ^a]", "^", "a", "b")
    check("(?x)[^ a]", "^", "a", "b")
    check("(?x)[a- z]", "m", "-", " ")
    check("(?x)[a -z]", "m", "-", " ")
    check("(?x)[+- ]]", "+", "0", "]", "-")
    check("(?x)[a- [bc]]", "a")
    check("(?x)[a -[bc]]", "a", "-", "b")
    check("(?x)[a& b]", "a", "b", "&")
    check("(?x)[a& &b]", "a", "b", "&")
    check("(?x)[a-z & & [^aeiou]]", "b", "a", "&")
    check("(?x)[\\v -a]", "-", "a", "\u000B", "b")
    check("(?x)a{2, 3}", "aa", "aaa", "a")
    check("(?x)a{ 2}", "aa")
    check("(?x)a+ ?b", "aab")
    check("(?x)( ?:a)", "a")
    check("(?x)(? :a)", "a")
    check("(?x)(? =a)a", "a")
    check("(?x)(?< =a)b", "b")
    check("(?x)(?<n a>x) \\k<n a>", "xx")
    check("(?x)(a)\\1 0", "aa0")
    check("(?x)\\x 4 1 \\u 0 0 4 2 \\0 1 0 3", "ABC")
    check("(?x)\\p {Lu}", "A", "a")
    check("(?x)\\p{ Lu}", "A", "a")
    check("(?x)\\p{Lu }", "A")
    check("(?x)\\p L", "a", "1")
    check("(?x)\\p{L} # a letter\n+", "abc", "a1")
    check("(?x)\\c ", "@")
    check("(?x)\\c#note", "@")
    check("(?x)\\c A", "\u0001")
