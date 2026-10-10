// A locale's display names, of the languages and regions Locale's constants name, in English; a
// null locale to show them in refused as the JDK refuses it, an empty code's name too.
import java.util.Locale

@main def run(): Unit =
  println(Locale.FRANCE.getDisplayCountry(Locale.US))
  println(Locale.GERMANY.getDisplayLanguage(Locale.US))
  println(Locale.US.getDisplayName(Locale.US))
  println(Locale.JAPANESE.getDisplayName(Locale.US))
  println(Locale.of("", "IT").getDisplayName(Locale.US))
  println(List(Locale.UK, Locale.CANADA, Locale.ITALY, Locale.JAPAN).map(_.getDisplayCountry(Locale.ENGLISH)))
  println(Locale.ROOT.getDisplayName(Locale.US).isEmpty)
  def attempt(label: String)(f: => String): Unit =
    try println(s"$label ${f}") catch case e: NullPointerException => println(s"$label NPE ${e.getMessage}")
  attempt("country")(Locale.FRANCE.getDisplayCountry(null))
  attempt("language")(Locale.FRANCE.getDisplayLanguage(null))
  attempt("name")(Locale.FRANCE.getDisplayName(null))
  attempt("empty country")(Locale.ENGLISH.getDisplayCountry(null))
