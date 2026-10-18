// Inputs joined element by element, the shape of tapir's `ParamConcat` over `TupleOps.Join` without its jar:
// an overloaded `in` asks for `Concat.Aux[I, J, IJ]` over the builder's input `I` the previous `in` left, and the
// instance folds the new input onto that tuple through `Join`, `FoldLeft`, `Step` and `Append`, each a given with a
// using clause whose target the divergence check measures. `IJ`, which the settlement leaves open
// (`Inferencing.interpolateTypeVars` leaves the nonvariant occurrence to the expected type), bounded by the tuple on
// both sides, is measured by its bounds read once, so that a builder's cost grows with its inputs and not with the
// bounds' paths. The inferred builders' types are exact.
trait Append[P, S]:
  type Out
  def apply(p: P, s: S): Out
object Append:
  type Aux[P, S, O] = Append[P, S] { type Out = O }
  given one[A, B]: Aux[Tuple1[A], B, (A, B)] = new Append[Tuple1[A], B] { type Out = (A, B); def apply(p: Tuple1[A], s: B) = (p._1, s) }
  given two[A, B, C]: Aux[(A, B), C, (A, B, C)] = new Append[(A, B), C] { type Out = (A, B, C); def apply(p: (A, B), s: C) = (p._1, p._2, s) }
  given three[A, B, C, D]: Aux[(A, B, C), D, (A, B, C, D)] = new Append[(A, B, C), D] { type Out = (A, B, C, D); def apply(p: (A, B, C), s: D) = (p._1, p._2, p._3, s) }
  given four[A, B, C, D, E]: Aux[(A, B, C, D), E, (A, B, C, D, E)] = new Append[(A, B, C, D), E] { type Out = (A, B, C, D, E); def apply(p: (A, B, C, D), s: E) = (p._1, p._2, p._3, p._4, s) }
  given five[A, B, C, D, E, F]: Aux[(A, B, C, D, E), F, (A, B, C, D, E, F)] = new Append[(A, B, C, D, E), F] { type Out = (A, B, C, D, E, F); def apply(p: (A, B, C, D, E), s: F) = (p._1, p._2, p._3, p._4, p._5, s) }
  given six[A, B, C, D, E, F, G]: Aux[(A, B, C, D, E, F), G, (A, B, C, D, E, F, G)] = new Append[(A, B, C, D, E, F), G] { type Out = (A, B, C, D, E, F, G); def apply(p: (A, B, C, D, E, F), s: G) = (p._1, p._2, p._3, p._4, p._5, p._6, s) }
  given seven[A, B, C, D, E, F, G, H]: Aux[(A, B, C, D, E, F, G), H, (A, B, C, D, E, F, G, H)] = new Append[(A, B, C, D, E, F, G), H] { type Out = (A, B, C, D, E, F, G, H); def apply(p: (A, B, C, D, E, F, G), s: H) = (p._1, p._2, p._3, p._4, p._5, p._6, p._7, s) }
  given eight[A, B, C, D, E, F, G, H, I]: Aux[(A, B, C, D, E, F, G, H), I, (A, B, C, D, E, F, G, H, I)] = new Append[(A, B, C, D, E, F, G, H), I] { type Out = (A, B, C, D, E, F, G, H, I); def apply(p: (A, B, C, D, E, F, G, H), s: I) = (p._1, p._2, p._3, p._4, p._5, p._6, p._7, p._8, s) }
  given nine[A, B, C, D, E, F, G, H, I, J]: Aux[(A, B, C, D, E, F, G, H, I), J, (A, B, C, D, E, F, G, H, I, J)] = new Append[(A, B, C, D, E, F, G, H, I), J] { type Out = (A, B, C, D, E, F, G, H, I, J); def apply(p: (A, B, C, D, E, F, G, H, I), s: J) = (p._1, p._2, p._3, p._4, p._5, p._6, p._7, p._8, p._9, s) }
  given ten[A, B, C, D, E, F, G, H, I, J, K]: Aux[(A, B, C, D, E, F, G, H, I, J), K, (A, B, C, D, E, F, G, H, I, J, K)] = new Append[(A, B, C, D, E, F, G, H, I, J), K] { type Out = (A, B, C, D, E, F, G, H, I, J, K); def apply(p: (A, B, C, D, E, F, G, H, I, J), s: K) = (p._1, p._2, p._3, p._4, p._5, p._6, p._7, p._8, p._9, p._10, s) }
  given eleven[A, B, C, D, E, F, G, H, I, J, K, L]: Aux[(A, B, C, D, E, F, G, H, I, J, K), L, (A, B, C, D, E, F, G, H, I, J, K, L)] = new Append[(A, B, C, D, E, F, G, H, I, J, K), L] { type Out = (A, B, C, D, E, F, G, H, I, J, K, L); def apply(p: (A, B, C, D, E, F, G, H, I, J, K), s: L) = (p._1, p._2, p._3, p._4, p._5, p._6, p._7, p._8, p._9, p._10, p._11, s) }

trait Step[T, A]:
  type Out
  def apply(t: T, a: A): Out
object Step:
  type Aux[T, A, O] = Step[T, A] { type Out = O }
  given step[T, A](using append: Append[T, A]): Aux[T, A, append.Out] =
    new Step[T, A] { type Out = append.Out; def apply(t: T, a: A) = append(t, a) }

trait FoldLeft[In, T]:
  type Out
  def apply(zero: In, t: T): Out
object FoldLeft:
  type Aux[In, T, O] = FoldLeft[In, T] { type Out = O }
  given t1[In, A](using s: Step[In, A]): Aux[In, Tuple1[A], s.Out] =
    new FoldLeft[In, Tuple1[A]] { type Out = s.Out; def apply(zero: In, t: Tuple1[A]) = s(zero, t._1) }

trait Join[P, S]:
  type Out
  def apply(p: P, s: S): Out
object Join:
  type Aux[P, S, O] = Join[P, S] { type Out = O }
  given join[P, S](using fold: FoldLeft[P, S]): Aux[P, S, fold.Out] =
    new Join[P, S] { type Out = fold.Out; def apply(p: P, s: S) = fold(p, s) }

trait Concat[T, U]:
  type Out
  def apply(t: T, u: U): Out
object Concat extends LowConcat:
  type Aux[T, U, TU] = Concat[T, U] { type Out = TU }
  given unitLeft[U]: Aux[Unit, U, Tuple1[U]] = new Concat[Unit, U] { type Out = Tuple1[U]; def apply(t: Unit, u: U) = Tuple1(u) }
trait LowConcat:
  given tupleAndSingle[T, U, TU](using j: Join.Aux[T, Tuple1[U], TU]): Concat.Aux[T, U, TU] =
    new Concat[T, U] { type Out = TU; def apply(t: T, u: U) = j(t, Tuple1(u)) }

final case class Input[A](name: String, read: String => A)
final case class Stream[A](name: String)
final case class Builder[I](names: List[String], read: Map[String, String] => I):
  def in[J, IJ](i: Input[J])(using c: Concat.Aux[I, J, IJ]): Builder[IJ] =
    Builder(names :+ i.name, m => c(read(m), i.read(m(i.name))))
  def in[J, IJ](s: Stream[J])(using c: Concat.Aux[I, J, IJ]): Builder[IJ] =
    Builder(names :+ s.name, m => c(read(m), null.asInstanceOf[J]))
object Builder:
  val empty: Builder[Unit] = Builder(Nil, _ => ())

final case class Id(value: String) extends AnyVal

object Joins:
  val id = Input("id", s => Id(s))
  val n = Input("n", _.toInt)
  val on = Input("on", s => s.toIntOption)
  val os = Input("os", s => Option(s).filter(_.nonEmpty))
  val ob = Input("ob", s => s.toBooleanOption)
  val l = Input("l", _.toLong)
  val s = Input("s", identity)
  val ol = Input("ol", s => s.toLongOption)
  val bo = Input("bo", _.toBoolean)
  val c = Input("c", _.head)
  val sh = Input("sh", _.toShort)
  val by = Input("by", _.toByte)
  val b4 = Builder.empty.in(id).in(n).in(on).in(os)
  val b6 = Builder.empty.in(id).in(n).in(on).in(os).in(ob).in(l)
  val b12 = Builder.empty.in(id).in(n).in(on).in(os).in(ob).in(l).in(s).in(ol).in(bo).in(c).in(sh).in(by)

@main def run(): Unit =
  val m = Map("id" -> "x", "n" -> "1", "on" -> "2", "os" -> "", "ob" -> "true", "l" -> "7", "s" -> "t", "ol" -> "-", "bo" -> "false",
    "c" -> "q", "sh" -> "3", "by" -> "4")
  val four: Builder[(Id, Int, Option[Int], Option[String])] = Joins.b4
  val six: Builder[(Id, Int, Option[Int], Option[String], Option[Boolean], Long)] = Joins.b6
  println(four.read(m))
  println(six.read(m))
  val twelve: Builder[(Id, Int, Option[Int], Option[String], Option[Boolean], Long, String, Option[Long], Boolean, Char, Short, Byte)] =
    Joins.b12
  println(twelve.read(m))
  println(twelve.names.mkString(","))
