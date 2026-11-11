package tsa

// Type selections in the type trees a body keeps (its ascriptions', an applied type's head), on each kind of qualifier:
// an object (`Shapes.Twice`), a package (`tsa.K`), packages in a row (`scala.collection.immutable.List`, from
// `_root_`), an object under two packages (`tsa.sub.Deep.T`), a val's path (`O.v.T`) and a parameter (`b.T`), in inline
// bodies, which the downstream's expansions read from the pickles, and in a plain one. scalac pickles each as SELECTtpt
// over its qualifier's tree (`TreePickler.pickleTree`'s Select case).
class K(val n: Int)

object Shapes:
  type Twice[X] = (X, X)

object O:
  object Inner:
    type T = Int
  val v: Inner.type = Inner

trait Box:
  type T
  def get: T

object Paths:
  inline def twice(x: Int): Shapes.Twice[Int] = ((x, x): Shapes.Twice[Int])
  inline def k(x: K): Int = (x: tsa.K).n
  inline def len(xs: List[Int]): Int = (xs: scala.collection.immutable.List[Int]).size
  inline def inc(x: Int): Int = (x: O.v.T) + 1
  inline def got(b: Box): b.T = (b.get: b.T)
  inline def rooted(xs: List[Int]): Int = (xs: _root_.scala.collection.immutable.List[Int]).size
  inline def deep(x: Int): Int = (x: tsa.sub.Deep.T)
  def plain(xs: List[Int]): Int = (xs: scala.collection.immutable.List[Int]).size + (K(1): tsa.K).n
  def plainDeep(xs: List[Int]): Int =
    val d: Int = (3: tsa.sub.Deep.T)
    (xs: _root_.scala.collection.immutable.List[Int]).size * 10 + d
