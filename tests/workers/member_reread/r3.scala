object R3:
  def resource: AnyRef = getClass.getResource("/x3.txt")
  def name: String = getClass.getName
