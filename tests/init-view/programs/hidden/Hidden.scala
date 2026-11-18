class Host {
  def unused(): Unit = {
    class Lost {
      val a = b
      val b = 1
    }
    scala.sys.error("unused")
  }
}
