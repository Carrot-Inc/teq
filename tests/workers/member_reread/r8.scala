object R8:
  def resource: AnyRef = getClass.getResource("/x8.txt")
  def name: String = getClass.getName
