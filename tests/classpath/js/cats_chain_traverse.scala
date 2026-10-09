// jars: cats-kernel-sjs cats-core-sjs
// Chain's traversal and instances from cats' bodies: `traverseViaChain` names
// `scala.math.package.min`, a package object selected as a term, and the instances the std's
// `Seq.newBuilder`, `java.lang.Double.compare` and `BigInt.int2bigInt`.
//> using dep org.typelevel::cats-core:2.13.0
import cats.*
import cats.data.Chain
import cats.syntax.all.*
object Main:
  def main(args: Array[String]): Unit =
    val c = Chain(1, 2, 3)
    println(c.traverse(x => Option(x + 1)))
    println(Traverse[Chain].traverse(c)(x => List(x, x * 10)).size)
    println(c.coflatMap(_.length).toString + " " + Monad[Chain].pure(4) + " " + Alternative[Chain].combineK(c, Chain(9)))
    println(c.foldMap(_.toString) + " " + c.collectFirstSome(x => if x > 1 then Some(x) else None) + " " + Traverse[Chain].size(c))
    println(Chain.fromSeq(1 to 30).traverse(x => if x > 0 then Some(x) else None).map(_.length))
    println(java.lang.Double.compare(1.5, 2.5) + " " + (BigInt(3) + BigInt.int2bigInt(1)) + " " + Seq.newBuilder[Int].addOne(1).addOne(2).result())
    println(Order[Double].compare(1.0, 2.0) + " " + Semigroup[Seq[Int]].combineAllOption(List(Seq(1), Seq(2, 3))) + " " + cats.kernel.instances.bigInt.catsKernelStdOrderForBigInt.next(BigInt(7)))
