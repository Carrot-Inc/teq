/*
rules = DisableSyntax
DisableSyntax.noWhileLoops = true
DisableSyntax.keywords = [if, for, match, try, new, this, val]
*/
// An end marker's name that is a keyword is that keyword's token, as scalameta's tokenizer scans it (`end while` an
// `Ident` and a `KwWhile`), so scalafix's `Keyword` extractor reports it where its setting disables the keyword; `given`
// is no keyword scalafix's DisableSyntax knows (`Keyword.all`), so `end given` is never reported. A case of teq's own,
// from scalafix's DisableSyntax and DisableSyntaxConfig at 41cf7983 (`Keyword.unapply`, `kwSet`).
package endmarkers

object EndMarkerKeywords:
  def loop(n: Int): Unit =
    var i = n
    while i > 0 do // assert: DisableSyntax.while
      i -= 1
    end while // assert: DisableSyntax.while
  def cond(b: Boolean): Int =
    if b then // assert: DisableSyntax.if
      1
    else
      2
    end if // assert: DisableSyntax.if
  def pick(x: Int): Int =
    x match // assert: DisableSyntax.match
      case 0 => 1
      case _ => 2
    end match // assert: DisableSyntax.match
  def each(): Unit =
    for k <- List(1) do // assert: DisableSyntax.for
      println(k)
    end for // assert: DisableSyntax.for
  def guard(): Int =
    try // assert: DisableSyntax.try
      1
    finally
      println("done")
    end try // assert: DisableSyntax.try
  def anon(): Object =
    new Object: // assert: DisableSyntax.new
      override def toString = "o"
    end new // assert: DisableSyntax.new
  class K(i: Int):
    def this() = // assert: DisableSyntax.this
      this(1) // assert: DisableSyntax.this
    end this // assert: DisableSyntax.this
  val (a, b) = // assert: DisableSyntax.val
    (1, 2)
  end val // assert: DisableSyntax.val
  given Ordering[K] with
    def compare(p: K, q: K) = 0
  end given
end EndMarkerKeywords
