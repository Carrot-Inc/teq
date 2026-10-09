import java.util.regex.Pattern

// `Pattern.quote` of a string holding `\E` closes the quote around it.
@main def main(): Unit =
  for s <- List("a\\Eb", "\\E", "x\\Q\\Ey", "plain.*") do
    val q = Pattern.quote(s)
    println(s"$q ${Pattern.compile(q).matcher(s).matches()} ${Pattern.compile(q).matcher(s + "!").matches()}")
