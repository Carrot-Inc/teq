/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Argument clauses as scalameta 4.17.0 lays them out (scalameta.txt): a `Term.ArgClause` from the token after the
// function (past its parentheses) to the application's end, a `using` one's `Mod.Using` after its values, an infix
// operation's right operand its clause beside an empty `Type.ArgClause`, an `Init`'s clauses each to its `)`; a type
// application's `Type.ArgClause`, `A | B` an infix type, an extractor's `Pat.ArgClause`, `xs*` a `Term.Repeated`.
object Clauses extends Exception("message"):
  val marker = List(1, 2) // assert: DumpLayout.layout
  val empty = List()
  val nested = List(1).map(_ + 1)
  val parenthesized = (identity[Int])(1)
  val given1 = summon[Ordering[Int]]
  val usingArg = List(2, 1).sorted(using Ordering.Int)
  val infix = 1 max 2
  val infixTuple = List(1) :+ (2)
  val typed = Map.empty[String, List[Int]]
  val union: Int | String = 1
  val inter: Product & Serializable = Some(1)
  val splice = List(List(1)*)
  val ascribed = List(List(1): _*)
  @deprecated("old", "1.0") val annotated = 1
  val Some(a) = Option(1): @unchecked
  val h :: t = List(1, 2): @unchecked
