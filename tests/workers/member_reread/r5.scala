object R5:
  def resource: AnyRef = getClass.getResource("/x5.txt")
  def name: String = getClass.getName
