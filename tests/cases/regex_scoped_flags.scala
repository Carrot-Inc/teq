//> using platform jvm
// The second code pass's `regex_scoped_flags` program.
import java.util.regex.Pattern
@main def run(): Unit =
  for (pattern, text) <- List(("(?iU:é)(?-U:é)", "Éé"), ("(?iU:é)(?-U:é)", "ÉÉ"), ("(?iU)\\p{Upper}", "ǅ"), ("(?i)\\p{Lt}", "A"), ("(?iU)é(?-U)é", "ÉÉ")) do
    println(Pattern.compile(pattern).matcher(text).matches())
