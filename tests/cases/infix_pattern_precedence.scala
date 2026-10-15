// Infix patterns grouped by their operators' precedence and associativity, as dotty's `infixPattern`
// groups them through `infixOps`: http4s's routes `Root / "tab" / IntVar(tab)` (left-associative `/`) and
// `Root / "search" :? Matcher(q)` (`/` binding tighter than `:?`), and right-associative `+:` and `::`.
object / :
  def unapply(s: String): Option[(String, String)] =
    val i = s.lastIndexOf('/')
    if i < 0 then None else Some((s.take(i), s.drop(i + 1)))
object :? :
  def unapply(s: String): Option[(String, String)] =
    val i = s.indexOf('?')
    if i < 0 then None else Some((s.take(i), s.drop(i + 1)))
object +: :
  def unapply(s: String): Option[(Char, String)] = s.headOption.map(c => (c, s.tail))
@main def run(): Unit =
  "root/tab/3" match
    case r / "tab" / n => println(s"$r $n")
  "root/search?q=x" match
    case r / "search" :? q => println(s"$r $q")
  "abc" match
    case a +: b +: rest => println(s"$a $b $rest")
  List(1, 2, 3) match
    case x :: y :: rest => println(s"$x $y $rest")
    case _ => println("short")
