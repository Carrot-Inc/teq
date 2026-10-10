// expect: 21:30: error: ambiguous given instances for Int: g3, g2, g1, g0, outer
// expect: 1 error found
// Nested imports of one owner stand at the level of the method's given however many there are
// (`ContextualImplicits.level`), and givens of distinct names are all candidates there: ambiguous,
// as scalac finds them at three, four and five imports alike.
object G0 { given g0: Int = 0 }
object G1 { given g1: Int = 1 }
object G2 { given g2: Int = 2 }
object G3 { given g3: Int = 3 }

@main def run(): Unit =
  given outer: Int = 9
  locally {
    import G0.given
    locally {
      import G1.given
      locally {
        import G2.given
        locally {
          import G3.given
          println(summon[Int])
        }
      }
    }
  }
