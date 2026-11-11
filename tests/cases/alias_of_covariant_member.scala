trait Fold[F[_]]:
  def toList[A](fa: F[A]): List[A]

object Impl:
  type Type[+A]
  def create[A](s: List[A]): Type[A] = s.asInstanceOf[Type[A]]

type Chain[+A] = Impl.Type[A]

given foldForChain: Fold[Chain] with
  def toList[A](fa: Chain[A]): List[A] = fa.asInstanceOf[List[A]]

trait Co[+F[_]]
def aliasAbove(x: Co[Impl.Type]): Co[Chain] = x
def aliasBelow(x: Co[Chain]): Co[Impl.Type] = x

def viaMember[A](xs: Impl.Type[A])(using f: Fold[Impl.Type]): List[A] = f.toList(xs)

@main def run(): Unit =
  println(viaMember(Impl.create(List(1, 2, 3))))
  println(summon[Fold[Impl.Type]].toList(Impl.create(List("a"))))
