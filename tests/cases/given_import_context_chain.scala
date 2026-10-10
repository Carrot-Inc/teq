// An import is a context of the search composed with the one outside it alone
// (`ContextualImplicits.level`, `combineEligibles`): of one owner it stands at that context's level,
// and the outer one's precedence beats it only where it beats the import's own (a definition any
// import, a named import a wildcard one); a second import of the same given beats the first.
object Values:
  given Int = 7
object G0:
  given Int = 0
object G1:
  given Int = 1
object A:
  given given_Int: Int = 1
object B:
  given Int = 2

def repeated: Int =
  given Int = 9
  locally {
    import Values.given
    locally {
      import Values.given
      summon[Int]
    }
  }

def distinct: Int =
  given Int = 9
  locally {
    import G0.given
    locally {
      import G1.given
      summon[Int]
    }
  }

def namedThenWild: List[Int] =
  given Int = 9
  locally {
    import A.given_Int
    val first = summon[Int]
    locally {
      import B.given
      val second = summon[Int]
      locally {
        import A.given_Int
        List(first, second, summon[Int])
      }
    }
  }

def valOwner: (Int, Int) =
  given Int = 9
  val x =
    import Values.given
    summon[Int]
  val y = locally {
    import Values.given
    summon[Int]
  }
  (x, y)

@main def run(): Unit =
  println(repeated)
  println(distinct)
  println(namedThenWild)
  println(valOwner)
