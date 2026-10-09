// A given whose result names a using parameter's member (`fold.Out`, the akka-http style
// tuple folds tapir's `ParamConcat` is built on) is matched with that member open and settled
// once the argument is found; the search reaches through three such levels here.
trait AppendOne[P, S]:
  type Out
  def apply(prefix: P, last: S): Out
object AppendOne:
  type Aux[P, S, O] = AppendOne[P, S] { type Out = O }
  implicit def t1[T1, L]: Aux[Tuple1[T1], L, Tuple2[T1, L]] = new AppendOne[Tuple1[T1], L]:
    type Out = Tuple2[T1, L]
    def apply(prefix: Tuple1[T1], last: L): Out = (prefix._1, last)

trait Case[A, B]:
  type Out
  def apply(a: A, b: B): Out
object Case:
  implicit def step[T, A](implicit append: AppendOne[T, A]): Case[T, A] { type Out = append.Out } =
    new Case[T, A]:
      type Out = append.Out
      def apply(a: T, b: A): Out = append(a, b)

trait FoldLeft[In, T]:
  type Out
  def apply(zero: In, tuple: T): Out
object FoldLeft:
  type Aux[In, T, Out0] = FoldLeft[In, T] { type Out = Out0 }
  implicit def f1[In, A](implicit f: Case[In, A]): Aux[In, Tuple1[A], f.Out] =
    new FoldLeft[In, Tuple1[A]]:
      type Out = f.Out
      def apply(zero: In, tuple: Tuple1[A]): Out = f(zero, tuple._1)

trait Join[P, S]:
  type Out
  def apply(p: P, s: S): Out
object Join:
  type JoinAux[P, S, O] = Join[P, S] { type Out = O }
  implicit def join[P, S](implicit fold: FoldLeft[P, S]): JoinAux[P, S, fold.Out] =
    new Join[P, S]:
      type Out = fold.Out
      def apply(p: P, s: S): Out = fold(p, s)

trait Concat[T, U]:
  type Out
object Concat:
  type Aux[T, U, TU] = Concat[T, U] { type Out = TU }
  implicit def single[T, U, TU](implicit tc: Join.JoinAux[Tuple1[T], Tuple1[U], TU]): Aux[T, U, TU] =
    new Concat[T, U] { type Out = TU }

case class Input[T](name: String):
  def and[U, TU](other: Input[U])(implicit c: Concat.Aux[T, U, TU]): Input[TU] = Input(name + "&" + other.name)
  def in[U](other: Input[U])(implicit c: Concat[T, U]): Input[c.Out] = Input(name + "+" + other.name)

object Main:
  def main(args: Array[String]): Unit =
    val a: Input[(Int, String)] = Input[Int]("i").and(Input[String]("s"))
    val b: Input[(Int, String)] = Input[Int]("i").in(Input[String]("s"))
    val j = Join.join[Tuple1[Int], Tuple1[String]]
    val r: (Int, String) = j(Tuple1(1), Tuple1("x"))
    println(a.name + " " + b.name + " " + r)
