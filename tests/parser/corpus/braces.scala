//> using platform js
trait Shape {
  def area: Double
  def describe: String = {
    val a = area
    s"shape with area $a"
  }
}

final case class Rect(w: Double, h: Double) extends Shape {
  def area: Double = w * h
}

enum Color {
  case Red, Green
  case Custom(hex: String)
}

object Geometry {
  def total(shapes: List[Shape]): Double = {
    shapes.map { s =>
      val a = s.area
      a
    }.foldLeft(0.0) { (acc, a) =>
      acc + a
    }
  }

  def name(c: Color): String = c match {
    case Color.Red => "red"
    case Color.Green => {
      val g = "green"
      g
    }
    case Color.Custom(hex) => hex
  }
}

extension (x: Int) {
  def twice: Int = x * 2
  def clamp(lo: Int, hi: Int): Int = {
    if x < lo then {
      lo
    } else if x > hi then {
      hi
    } else {
      x
    }
  }
}

def wrap(n: Int)(body: => Int): Int = body + n

def both(a: Int)(f: Int => Int)(g: Int => Int): Int = g(f(a))

def effect(body: => Unit): () => Unit = () => body

@main def main(): Unit = {
  val shapes = List(Rect(1, 2), Rect(3, 4))
  println(Geometry.total(shapes))
  println(shapes.head.describe)
  println(Geometry.name(Color.Green) + Geometry.name(Color.Custom("#fff")))

  val pairs = List(1 -> "a", 2 -> "b")
  println(pairs.map { case (n, s) => s * n })
  println(pairs.map { case (n, s) =>
    val r = s * n
    r + "!"
  })
  println(pairs.map({ case (n, _) => n }).sum)
  println(List(1, 2, 3).map { _ + 1 })
  println(List(1, 2, 3).map { x => x * x }.filter { _ > 1 })

  println(wrap(1) { 41 })
  println(wrap(1) {
    val x = 20
    x * 2
  })
  println(both(1) { _ + 1 } { x =>
    x * 10
  })

  val add = (a: Int, b: Int) => {
    val s = a + b
    s
  }
  println(add(2, 3))
  println(List(3, 1, 2).sortWith((a, b) => {
    a < b
  }))

  var count = 0
  val bump = effect { count += 1 }
  bump(); bump()
  println(count)

  val nested = { val a = 1; val b = 2; a + b }
  println(nested)
  println(5.twice + 50.clamp(0, 10) + { 100 })

  val sum = for {
    a <- List(1, 2)
    b <- List(10, 20)
    if a + b != 21
  } yield {
    a + b
  }
  println(sum)

  var i = 0
  while (i < 3) do {
    i += 1
  }
  println(i)

  val empty: () => Unit = () => {}
  empty()
  println(Option(3).fold("none") { v => s"some $v" })
}
