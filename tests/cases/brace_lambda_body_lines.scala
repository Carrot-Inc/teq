// The body of a lambda or of a case that begins on the line after `{ x =>` or `{ case x =>` is the braces' own
// statements, at the width of its first line: an `else` or a `yield` after the closing brace goes on with the
// expression the braces are an argument of, and the same holds for such braces in a for's generator and in an
// argument list. scalac prints the same lines.
def afterElse(c: Boolean, xs: List[Int]): List[Int] =
  if c then xs.map { x =>
    val y = x + 1
    y * 2
  } else xs.map { case x =>
    val y = x - 1
    y
  }

def inGenerator(xs: List[Int]): List[Int] =
  for
    x <- xs.map { x =>
      val y = x * 10
      y
    }
    z <- List(x).collect { case w =>
      val v = w + 1
      v
    }
  yield z

def inArguments(xs: List[Int]): Int =
  xs.foldLeft(0)({ (a, x) =>
    val b = a + x
    b
  }) + List(1).map({ case x =>
    val y = x + 100
    y
  }).head

def nested(xs: List[Int]): List[String] =
  xs.map { x =>
    val inner = List(x, x + 1).map { y =>
      val z = y * 2
      z
    }
    inner.mkString("-")
  }

@main def main(): Unit =
  println((afterElse(true, List(1, 2)), afterElse(false, List(1, 2))))
  println(inGenerator(List(1, 2)))
  println(inArguments(List(1, 2, 3)))
  println(nested(List(1, 3)))
