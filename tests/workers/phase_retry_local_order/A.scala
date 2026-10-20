object A:
  val ready: Int = SourceMacro.value
  def result: String =
    class C:
      def x: Int = M.next
      val y: Int = M.next
    val z = M.next
    val c = new C
    s"${c.x},${c.y},$z"
