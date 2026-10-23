// The corpus's line of scalameta 4.17.4's scanner over a program (`#!regions`, read by `nodes::tests::scalameta_regions`,
// written by support/ScalametaCorpus.scala): the tokens the scanner makes, in the order its parser reads them
// (`ScannerTokens.nextToken` through `LazyTokenIterator`), each by a letter (`I` an `Indent`, `O` an `Outdent`, `L` a
// line break, `F` an `LFLF`, `X` an `InfixLF`), its start and the widths of the indented regions it stands in, the
// innermost first. In scalameta's package, whose iterator and regions are private to it.
package scala.meta.internal.parsers

import scala.meta._
import scala.meta.tokens.Token

object RegionStream {
  def line(text: String, dialect: Dialect): String = scala.util.Try {
    val tokens = dialect(Input.VirtualFile("W.scala", text)).tokenize.get
    val it = LazyTokenIterator(new ScannerTokens(tokens)(dialect))
    val shown = List.newBuilder[String]
    while (it.hasCurr && !it.currToken.is[Token.EOF]) {
      val letter = it.currToken match {
        case _: Token.Indentation.Indent => "I"
        case _: Token.Indentation.Outdent => "O"
        case _: Token.LFLF => "F"
        case _: Token.InfixLF => "X"
        case _: Token.AtEOL => "L"
        case _ => null
      }
      if (letter != null)
        shown += letter + it.currToken.start + "/" + it.currRegions.collect { case r: RegionIndent => r.indent }.mkString(".")
      it.next()
    }
    shown.result().mkString(" ")
  }.getOrElse("refused")
}
