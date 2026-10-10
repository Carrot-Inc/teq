// A locale's display names, of the languages and regions Locale's constants name, in English.
import java.util.Locale

@main def run(): Unit =
  println(Locale.FRANCE.getDisplayCountry(Locale.US))
  println(Locale.GERMANY.getDisplayLanguage(Locale.US))
  println(Locale.US.getDisplayName(Locale.US))
  println(Locale.JAPANESE.getDisplayName(Locale.US))
  println(Locale.of("", "IT").getDisplayName(Locale.US))
  println(List(Locale.UK, Locale.CANADA, Locale.ITALY, Locale.JAPAN).map(_.getDisplayCountry(Locale.ENGLISH)))
  println(Locale.ROOT.getDisplayName(Locale.US).isEmpty)
