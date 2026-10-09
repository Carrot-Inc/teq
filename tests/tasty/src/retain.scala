package fix.retain

// Inline overrides of two overloads: each keeps a retained body of its own.
trait Fmt:
  def show(x: Int): String
  def show(x: String): String

object Loud extends Fmt:
  inline override def show(x: Int): String = "int " + (x + 1)
  inline override def show(x: String): String = "str " + x.length

// Overloads that erase apart by their arrays' elements only.
trait Sizes:
  def size(x: Array[Int]): String
  def size(x: Array[String]): String

object Sized extends Sizes:
  inline override def size(x: Array[Int]): String = "ints " + x.sum
  inline override def size(x: Array[String]): String = "strings " + x.mkString

// Overloads that erase apart by their nested arrays' elements only.
trait Grids:
  def cells(x: Array[Array[Int]]): String
  def cells(x: Array[Array[String]]): String

object Grid extends Grids:
  inline override def cells(x: Array[Array[Int]]): String = "ints " + x.map(_.sum).sum
  inline override def cells(x: Array[Array[String]]): String = "strings " + x.map(_.mkString).mkString
