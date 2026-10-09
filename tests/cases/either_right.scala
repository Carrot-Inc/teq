// `Either.right`, the projection scala-library keeps next to `left`.
package eitherright

object Main:
  def main(args: Array[String]): Unit =
    val r: Either[String, Int] = Right(3)
    val l: Either[String, Int] = Left("no")
    println(r.right.map(_ + 1).toString + " " + l.right.map(_ + 1) + " " + r.right.toOption + " " + l.right.getOrElse(0) + " " + r.right.get)
    println(r.right.flatMap(x => Right(x * 2)).toString + " " + l.left.map(_.toUpperCase) + " " + r.right.exists(_ > 2) + " " + l.right.forall(_ > 2) + " " + r.right.toSeq)
