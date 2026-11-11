// expect: 9:21: error: Found:    Option[Nothing]
// expect: 10:21: error: Found:    List[Nothing]
// expect: 2 errors found
// An inferred TypeApply keeps its synthetic envelope over the function tree (Typer.adapt).
// Its point is the beginning of the selection, rather than the selected name. The adjacent
// capture records scalac 3.8.4's zero-based columns; diagnostic headers here use one-based columns.
object Main:
  def main(args: Array[String]): Unit =
    val s: String = Option.empty
    val t: String = List.empty
