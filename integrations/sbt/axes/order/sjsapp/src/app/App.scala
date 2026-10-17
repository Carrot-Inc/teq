package app

object App {
  def doubled: Int = core.Core.twice(core.Core.n)
  given Int = core.Core.`given`
}
