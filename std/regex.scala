package scala.util.matching

// A java.util.regex pattern run by the JS engine: leading (?i), (?s) and (?m) become flags, and
// \\s, $, \\A, \\z, \\Z, \\h and \\Q..\\E are rewritten to mean what they mean in Java.
final class Regex(val regex: String, wholeInput: Boolean = true):
  def pattern: java.util.regex.Pattern = java.util.regex.Pattern.compile(regex)
  def unanchored: Regex = new Regex(regex, false)
  def anchored: Regex = new Regex(regex, true)
  // The inputs are CharSequences, as scala-library's: the engine reads their text.
  def findFirstMatchIn(source: CharSequence): Option[Regex.Match] = Regex.wrap(source, Regex.exec(regex, "", source.toString))
  def findFirstIn(source: CharSequence): Option[String] = findFirstMatchIn(source).map(m => m.matched)
  def findAllMatchIn(source: CharSequence): Iterator[Regex.Match] =
    arrayIterator(Regex.all(regex, source.toString).map(m => new Regex.Match(source, m)))
  def findAllIn(source: CharSequence): Iterator[String] = findAllMatchIn(source).map(m => m.matched)
  def findPrefixMatchOf(source: CharSequence): Option[Regex.Match] = Regex.wrap(source, Regex.exec(regex, "y", source.toString))
  def findPrefixOf(source: CharSequence): Option[String] = findPrefixMatchOf(source).map(m => m.matched)
  def matches(source: CharSequence): Boolean = !js.isNull(Regex.exec(regex, "f", source.toString))
  def replaceAllIn(target: CharSequence, replacement: String | (Regex.Match => String)): String =
    Regex.replace(regex, target.toString, replacement, true, m => new Regex.Match(target, m))
  def replaceFirstIn(target: CharSequence, replacement: String): String =
    Regex.replace(regex, target.toString, replacement, false, m => new Regex.Match(target, m))
  def replaceSomeIn(target: CharSequence, replacer: Regex.Match => Option[String]): String =
    Regex.replace(regex, target.toString, (m: Regex.Match) => replacer(m).getOrElse(Regex.quoteReplacement(m.matched)), true, m => new Regex.Match(target, m))
  def split(toSplit: CharSequence): Array[String] = Regex.splitBy(regex, toSplit.toString, 0)
  // What `case r(a, b)` binds: the groups of a match of the whole input (any match when unanchored).
  def unapplySeq(source: CharSequence): Option[List[String]] =
    Regex.wrap(source, Regex.exec(regex, if wholeInput then "f" else "", source.toString)).map(m => m.subgroups)
  override def toString: String = regex

object Regex:
  final class Match(val source: CharSequence, raw: Any):
    def matched: String = Regex.groupOf(raw, 0)
    def group(id: Int | String): String = Regex.groupOf(raw, id)
    def groupCount: Int = Regex.groupCountOf(raw)
    def subgroups: List[String] = List.range(1, groupCount + 1).map(i => Regex.groupOf(raw, i))
    def start: Int = Regex.startOf(raw)
    def end: Int = start + matched.length
    def before: CharSequence = source.subSequence(0, start)
    def after: CharSequence = source.subSequence(end, source.length)
    override def toString: String = matched

  def quote(text: String): String = "\\Q" + text + "\\E"
  @js("$1.replace(/[\\\\$]/g, \"\\\\$&\")")
  @jvm("$1 invokestatic java/util/regex/Matcher.quoteReplacement(Ljava/lang/String;)Ljava/lang/String;")
  def quoteReplacement(text: String): String

  def wrap(source: CharSequence, raw: Any): Option[Match] = if js.isNull(raw) then None else Some(new Match(source, raw))
  @js("$reExec($1, $2, $3)")
  @jvm("rt $1 $2 $3 rtcall regexExec(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")
  def exec(regex: String, mode: String, source: String): Any
  @js("$reAll($1, $2)")
  @jvm("rt $1 $2 rtcall regexAll(Ljava/lang/String;Ljava/lang/String;)[Ljava/lang/Object;")
  def all(regex: String, source: String): Array[Any]
  @js("$reReplace($1, $2, $3, $4, $5)")
  @jvm("rt $1 $2 $3:L $4:Z $5 rtcall regexReplace(Ljava/lang/String;Ljava/lang/String;Ljava/lang/Object;ZLscala/Function1;)Ljava/lang/String;")
  def replace(regex: String, target: String, replacement: String | (Match => String), all: Boolean, wrap: Any => Match): String
  @js("$reSplit($1, $2, $3)")
  @jvm("rt $2 $1 $3:I rtcall regexSplit(Ljava/lang/String;Ljava/lang/String;I)[Ljava/lang/String;")
  def splitBy(regex: String, source: String, limit: Int): Array[String]
  // An unmatched group is null, as in Java.
  @js("((typeof $2 === \"number\" ? $1[$2] : $1.groups === undefined ? undefined : $1.groups[$2]) ?? null)")
  @jvm("rt $1:L $2:L rtcall regexGroup(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/String;")
  def groupOf(raw: Any, id: Int | String): String
  @js("($1.length - 1)")
  @jvm("$1:L checkcast java/util/regex/MatchResult invokeinterface java/util/regex/MatchResult.groupCount()I")
  def groupCountOf(raw: Any): Int
  @js("$1.index")
  @jvm("$1:L checkcast java/util/regex/MatchResult invokeinterface java/util/regex/MatchResult.start()I")
  def startOf(raw: Any): Int
