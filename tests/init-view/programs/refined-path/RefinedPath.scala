class RefinedPath {
  val a = {
    println("keep")
    (0: cell.T)
  }
  val cell: { type T = Int } = new { type T = Int }
}
