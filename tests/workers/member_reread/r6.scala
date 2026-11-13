object R6:
  def resource: AnyRef = getClass.getResource("/x6.txt")
  def name: String = getClass.getName
