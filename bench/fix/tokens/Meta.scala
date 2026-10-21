//> using scala 3.8.4
//> using dep org.scalameta::scalameta:4.17.3
// The token classes scalameta 4.17.3 gives a file in the Scala 3 dialect with its default TokenizerOptions: `tokenize`'s
// raw stream, or with `tree` the parsed Source's `tree.tokens` (what scalafix's DisableSyntax reads, `doc.tree.tokens`).
// One line per token, `<class>|<start>|<end>|<text>`; the first line the options.
import scala.meta.*
@main def meta(path: String, mode: String): Unit =
  val text = new String(java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(path)), "UTF-8")
  val input = Input.VirtualFile(path, text)
  println("options|" + summon[scala.meta.tokenizers.TokenizerOptions])
  val tokens = mode match
    case "tree" => dialects.Scala3(input).parse[Source].get.tokens
    case _ => dialects.Scala3(input).tokenize.get
  for t <- tokens do
    val name = t.getClass.getName.stripPrefix("scala.meta.tokens.Token$").replace('$', '.').stripSuffix(".")
    println(s"$name|${t.start}|${t.end}|${t.text.replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t").replace("\f", "\\f")}")
