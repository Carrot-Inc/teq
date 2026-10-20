// lila: relay/RelayPlayerEnrich.scala:120/143 (`games.foldLeft(Vector.empty -> Nil): case ((games, ambiguous), game) => ...`)
// The expected result type of foldLeft constrains its B before the zero `Vector.empty -> Nil` is
// typed, so B is (Vector[Game], List[Amb]), not (Vector[Nothing], Nil.type).
case class Game(n: Int)
case class Amb(s: String)

def update(g: Game): (Game, List[Amb]) = (g, List(Amb(g.n.toString)))

def updateAll(games: Vector[Game]): (Vector[Game], List[Amb]) =
  games.foldLeft(Vector.empty -> Nil):
    case ((games, ambiguous), game) =>
      val (g, ambi) = update(game)
      (games :+ g) -> (ambi ::: ambiguous)

def updateTags(tags: String): (String, List[Amb]) =
  List(1, 2).foldLeft(tags -> Nil):
    case ((tags, ambiguous), c) =>
      val newAmbiguous = if c > 1 then Amb(tags) :: ambiguous else ambiguous
      (tags + c, newAmbiguous)

@main def run() =
  println(updateAll(Vector(Game(1), Game(2))))
  println(updateTags("t"))
