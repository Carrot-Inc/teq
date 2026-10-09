trait Functor[F[_]]:
  extension [A](fa: F[A]) def fmap[B](f: A => B): F[B]

trait Applicative[F[_]] extends Functor[F]:
  def pure[A](a: A): F[A]
  extension [A](fa: F[A]) def map2[B, C](fb: F[B])(f: (A, B) => C): F[C]

given Applicative[Option] with
  def pure[A](a: A): Option[A] = Some(a)
  extension [A](fa: Option[A])
    def fmap[B](f: A => B): Option[B] = fa.map(f)
    def map2[B, C](fb: Option[B])(f: (A, B) => C): Option[C] =
      fa.flatMap(a => fb.map(b => f(a, b)))

given Applicative[List] with
  def pure[A](a: A): List[A] = List(a)
  extension [A](fa: List[A])
    def fmap[B](f: A => B): List[B] = fa.map(f)
    def map2[B, C](fb: List[B])(f: (A, B) => C): List[C] =
      fa.flatMap(a => fb.map(b => f(a, b)))

given eitherFunctor[E]: Functor[[X] =>> Either[E, X]] with
  extension [A](fa: Either[E, A]) def fmap[B](f: A => B): Either[E, B] = fa.map(f)

def sequence[F[_], A](xs: List[F[A]])(using app: Applicative[F]): F[List[A]] =
  xs.foldRight(app.pure(List.empty[A]))((fa, acc) => fa.map2(acc)((a, as) => a :: as))

def incAll[F[_]: Functor](fa: F[Int]): F[Int] = fa.fmap(_ + 1)

enum Tree[+A]:
  case Leaf
  case Node(left: Tree[A], value: A, right: Tree[A])

  def size: Int = this match
    case Leaf => 0
    case Node(l, _, r) => l.size + 1 + r.size

  def map[B](f: A => B): Tree[B] = this match
    case Leaf => Leaf
    case Node(l, v, r) => Node(l.map(f), f(v), r.map(f))

  def toList: List[A] = this match
    case Leaf => Nil
    case Node(l, v, r) => l.toList ++ (v :: r.toList)

def insert[A](tree: Tree[A], x: A)(using ord: Ordering[A]): Tree[A] = tree match
  case Tree.Leaf => Tree.Node(Tree.Leaf, x, Tree.Leaf)
  case Tree.Node(l, v, r) =>
    if ord.lt(x, v) then Tree.Node(insert(l, x), v, r)
    else Tree.Node(l, v, insert(r, x))

given Functor[Tree] with
  extension [A](fa: Tree[A]) def fmap[B](f: A => B): Tree[B] = fa.map(f)

type Result[A] = Either[String, A]

def validate(n: Int): Result[Int] = if n >= 0 then Right(n) else Left(s"negative: $n")

class Box[T](val value: T):
  def map[U](f: T => U): Box[U] = Box(f(value))
  def zip[U](other: Box[U]): Box[(T, U)] = Box((value, other.value))
  override def toString: String = s"Box($value)"

def first[A, B](pair: (A, B)): A = pair._1
def swap[A, B](pair: (A, B)): (B, A) = (pair._2, pair._1)
def compose[A, B, C](f: A => B, g: B => C): A => C = a => g(f(a))

@main def run(): Unit =
  println(sequence(List(Option(1), Option(2), Option(3))))
  println(sequence(List(Option(1), None)))
  println(sequence(List(List(1, 2), List(3))))
  println(incAll(List(1, 2, 3)))
  println(incAll(Option(41)))
  val e: Either[String, Int] = Right(1)
  println(e.fmap(_ + 1))
  val bad: Either[String, Int] = Left("boom")
  println(bad.fmap(_ + 1))
  val tree = List(5, 2, 8, 1).foldLeft[Tree[Int]](Tree.Leaf)((t, x) => insert(t, x))
  println(tree.size)
  println(tree.toList)
  println(tree.map(_ * 10).toList)
  println(incAll(tree).toList)
  println(validate(3))
  println(validate(-3))
  println(Box(2).map(_ + 1).zip(Box("x")))
  println(first((1, "a")))
  println(swap((1, "a")))
  println(compose((x: Int) => x + 1, (y: Int) => y.toString + "!")(9))
