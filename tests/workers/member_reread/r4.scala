object R4:
  def resource: AnyRef = getClass.getResource("/x4.txt")
  def name: String = getClass.getName
