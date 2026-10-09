import scala.util.matching.Regex

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

def arr(parts: Array[String]): String = parts.mkString("[", ";", "]") + parts.length.toString

@main def main(): Unit =
  show("split", arr("a,b,,c,,".split(",")), arr(",a".split(",")), arr(",".split(",")), arr("".split(",")), arr("abc".split(",")))
  show("split char", arr("com.example.Name".split('.')), "com.example.Name".split('.').last, arr("a|b".split('|')))
  show("split regex", arr("a  b \t c".split("\\s+")), arr("1.2.3".split("\\.")), arr("a1b22c".split("\\d+")), arr("k: v".split(":\\s*")))
  show("split words", arr("--acme-theme-x".split("--acme-theme-")), arr("a, b;c".split("[,;]\\s*")), arr("abc".split("")))
  show("split limit", arr("a,b,c,d".split(",", 2)), arr("a,b,,".split(",", -1)), arr("a1b2c3".split("\\d", 3)))
  show("split then", "x=1&y=2".split("&").flatMap(_.split("=").headOption).toList, "1, 2,zz".split(",").flatMap(_.trim.toIntOption).toSet)
  show("split array ops", "b,a,c".split(",").sorted.toList, "a:b".split(":").map(_.toUpperCase).mkString("+"), "a b".split(" ").map(_.capitalize).mkString(" "))

  show("replaceAll", "a1b22".replaceAll("\\d+", "#"), "John Smith".replaceAll("(\\w+) (\\w+)", "$2, $1"), "x.y".replaceAll(".", "-"))
  show("replaceAll flags", "Main STREET and street".replaceAll("(?i)Street\\b", "St"), "a\nb".replaceAll("(?s)a.b", "ok"), "price $5".replaceAll("\\$", "USD "))
  show("replaceAll special", "a b".replaceAll(" ", "+"), "a.b".replaceAll("\\.", "\\\\"), "abc".replaceAll("", "-"), "file.tar.gz".replaceAll("\\.[^.]*$", ""))
  show("replaceAll spaces", "+1 (555) 010".replaceAll("\\s", ""), "(555) 010-99".replaceAll("[-_.()]", ""), "12px".replaceAll("[^\\d.]", ""))
  show("replaceFirst", "+1555+1".replaceFirst("^\\+1", ""), "aaa".replaceFirst("a", "b"), "abc".replaceFirst("z", "y"))
  show("trace", "internal-x.js?t=123:45:6 more".replaceAll(raw"internal-.*js\?t=\d*:(\d*:\d*)", "$1"))
  show("matches", "12345".matches("\\d+"), "123a".matches("\\d+"), "abc-def".matches("^[a-z-]+$"), "ab".matches("a"), "AB".matches("(?i)ab"))

  val activity = "activity/(\\d+)$".r
  show("findFirstIn", activity.findFirstIn("/x/activity/42"), activity.findFirstIn("/activity/42/x"), "\\d+".r.findFirstIn("a12b345"))
  show("findFirstMatchIn", activity.findFirstMatchIn("/x/activity/42").map(_.group(1)), activity.findFirstMatchIn("none").map(_.group(1)))
  val version = raw"(?i)\bandroid[ /-]?(\d[\w.]*)?".r
  show("optional group", version.findFirstMatchIn("Linux; ANDROID 14.1; x").flatMap(m => Option(m.group(1))), version.findFirstMatchIn("android;").flatMap(m => Option(m.group(1))), version.findFirstMatchIn("ios"))
  val hsl = """hsl\(\s*([\d.]+)(deg)?\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%\s*\)""".r
  val m = hsl.findFirstMatchIn("color: hsl(120.5, 50%, 25%);").get
  show("match", m.matched, m.group(1), m.group(3), m.group(4), m.groupCount, m.start, m.end, m.before, m.after, m.subgroups)
  show("match null group", Option(m.group(2)), m.toString)
  show("findAllIn", "\\d+".r.findAllIn("a1b22c333").toList, "x".r.findAllIn("abc").toList, "[aeiou]".r.findAllMatchIn("education").map(_.start).toList)
  show("named group", raw"(?<key>\w+)=(?<value>\w+)".r.findFirstMatchIn("a=b").map(g => g.group("value") + g.group("key")))
  show("regex matches", "^(-?\\d+)$".r.matches("-12"), "^(-?\\d+)$".r.matches("1.2"), "ab".r.matches("abc"), "a|ab".r.matches("ab"))
  val camel = "[A-Z\\d]".r.replaceAllIn("someCamelCase2", (g: Regex.Match) => "_" + g.group(0).toLowerCase)
  show("replaceAllIn", camel, "_".r.replaceAllIn(camel, " "), "(a)(b)".r.replaceAllIn("xaby", "$2$1"), "\\d".r.replaceFirstIn("a1b2", "#"))
  show("replaceSomeIn", "\\d".r.replaceSomeIn("a1b2c3", g => if g.matched == "2" then None else Some("<" + g.matched + ">")))
  show("prefix", "\\d+".r.findPrefixOf("12ab"), "\\d+".r.findPrefixOf("ab12"), "\\w".r.findPrefixMatchOf("xy").map(_.end))
  show("unapplySeq", "Master\\((.+)\\)".r.unapplySeq("Master(wine)"), "Master\\((.+)\\)".r.unapplySeq("xMaster(wine)"), "(\\d)(\\d)".r.unanchored.unapplySeq("a12b"))
  show("regex split", arr("\\s*,\\s*".r.split("a , b,c")), "ab+".r.toString, "ab+".r.regex)
  show("quote", Regex.quote("a.b").r.findFirstIn("xa.bx"), Regex.quote("a.b").r.findFirstIn("xaxbx"), "x".r.replaceAllIn("axb", Regex.quoteReplacement("$1\\")))
  show("line end", "c$".r.findFirstIn("abc\n").isDefined, "^b".r.findFirstIn("a\nb").isDefined, "(?m)^b".r.findFirstIn("a\nb").isDefined, "a\\s".r.findFirstIn("a ").isDefined)

  show("toIntOption", "12".toIntOption, "-7".toIntOption, "+7".toIntOption, "".toIntOption, "1.5".toIntOption, "2147483648".toIntOption, "-2147483648".toIntOption, " 1".toIntOption, "-".toIntOption)
  show("toLongOption", "9223372036854775807".toLongOption, "9223372036854775808".toLongOption, "x".toLongOption, "-5".toLongOption)
  show("toDoubleOption", "1.5".toDoubleOption.map(_ * 2 == 3.0), "1e5".toDoubleOption.map(_ == 100000.0), "".toDoubleOption, "0x10".toDoubleOption, "1,5".toDoubleOption, " 2.5 ".toDoubleOption.isDefined, ".5".toDoubleOption.isDefined, "5.".toDoubleOption.isDefined, "1d".toDoubleOption.isDefined, "abc".toDoubleOption)
  show("toDoubleOption special", "NaN".toDoubleOption.map(_.isNaN), "Infinity".toDoubleOption.map(_ > 0), "-Infinity".toDoubleOption.map(_ < 0), "infinity".toDoubleOption, "1e".toDoubleOption, "--1".toDoubleOption)
  show("toBooleanOption", "true".toBooleanOption, "FALSE".toBooleanOption, "yes".toBooleanOption, "True".toBoolean)
  show("toLong", "123456789012".toLong + 1L, "-5".toLong, "42".toInt, "3.25".toDouble * 2 == 6.5)

  show("strip", "prefix-body".stripPrefix("prefix-"), "body".stripPrefix("x"), "file.scala".stripSuffix(".scala"), "file".stripSuffix(".scala"))
  show("ignore case", "Hello".equalsIgnoreCase("hELLO"), "Hello".equalsIgnoreCase("Help"), "a".compareToIgnoreCase("B") < 0, "b".compareTo("a") > 0, "apple".compareTo("apricot"))
  show("repeat", "ab".repeat(3), "ab" * 2, "x".repeat(0), "-".concat(">"))
  show("grouped", "1234567".grouped(3).toList, "abc".sliding(2).toList, "1234567".reverse.grouped(3).mkString(",").reverse)
  show("foldLeft", "abc".foldLeft(0)((acc, c) => acc * 31 + c.toInt), "hello".foldLeft("")((acc, c) => c.toString + acc))
  show("lines", "a\nb\r\nc".linesIterator.toList, "one".linesIterator.toList)
  show("chars", "hello".find(_ > 'h'), "hello".indexWhere(_ == 'l'), "hello".lastOption, "".lastOption, "hello".takeWhile(_ != 'l'), "hello".dropWhile(_ != 'l'))
  show("chars more", "hello".span(_ != 'l'), "hello".splitAt(2), "a1b2".partition(_.isDigit), "a1b2".filterNot(_.isDigit), "banana".distinct, "banana".sorted)
  show("conversions", "abc".toList, "abc".toVector, "aab".toSet.size, "abc".toSeq.reverse.mkString, "ab".iterator.map(_.toUpper).mkString, "ab".toCharArray.length)
  show("misc", "abc".slice(1, 2), "abc".slice(-1, 10), "ab".padTo(4, '.'), "a-b".flatMap(c => if c == '-' then "__" else c.toString), "hello".indexOf("l", 3), "  ".isBlank, " x ".strip)
  show("margin",
    """|first
       |  second
       |third""".stripMargin)
  show("format", "%d items at %.2f (%s)".format(3, 9.5, "ok"), "%5d|%-5d|%05d".format(42, 42, 42), "%x %X %o".format(255, 255, 8), "%2$s %1$s".format("a", "b"))

  val scaled = 1234.5678
  val unit = "MB"
  val count = 7
  val big = 1234567890123L
  show("f double", f"$scaled%.1f $unit", f"$scaled%.0f", f"$scaled%.2f", f"$scaled%10.3f|", f"$scaled%-10.1f|", f"$scaled%,.2f", f"$scaled%e")
  show("f rounding", f"${1.005}%.2f", f"${2.5}%.0f", f"${0.125}%.2f", f"${-0.001}%.2f", f"${1e21}%.1f", f"${1.0e-5}%.3f", f"${99.995}%.2f", f"${0.5}%5.0f|")
  show("f int", f"$count%02d", f"$count%03d", f"$count%012d", f"${-count}%04d", f"$count%+d", f"$big%,d", f"$big%d", f"${255}%02x", f"${-1}%x", f"${10}%5d|")
  show("f color", f"#${255}%02x${0}%02x${128}%02x", f"#${255}%02X")
  show("f mixed", f"$unit%s and $unit and $count%% done", f"$unit%5s|$unit%-5s|", f"${true}%b ${'c'}%c", f"no args", f"a%nb")
  show("f uuid", f"00000000-0000-0000-0000-${42}%012d")

  show("char", 'a'.isLetter, '1'.isLetter, '1'.isDigit, '_'.isLetterOrDigit, 'A'.isUpper, 'a'.isUpper, ' '.isWhitespace, '\u0007'.isControl, 'a'.isControl)
  show("char digit", 'f'.asDigit, '7'.asDigit, 'Z'.asDigit, ('a' + 1).toChar, 'a'.toUpper, 'Q'.toLower, 'a'.toInt)

  val sb = new StringBuilder
  sb.append("hello").append(' ').append(42)
  sb ++= "!"
  sb += '?'
  show("builder", sb.toString, sb.length, sb.charAt(1), sb.indexOf("42"), sb.nonEmpty, sb.result(), sb.reverse.toString)
  sb.insert(0, ">> ")
  sb.deleteCharAt(sb.length - 1)
  sb.setLength(8)
  show("builder edits", sb.toString, sb.isEmpty, new StringBuilder("init").append(1).toString)
  sb.clear()
  show("builder cleared", sb.isEmpty, sb.toString)
  show("mkString", "abc".mkString(","), "abc".mkString, "abc".padTo(5, '.').mkString("-"))
  show("java parens", "Abc".toLowerCase(), "Abc".toUpperCase(), " x ".trim() + "|", List(" A ", "b").map(_.trim()).map(_.toUpperCase()))
