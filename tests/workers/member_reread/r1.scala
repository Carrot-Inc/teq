object R1:
  def resource: AnyRef = getClass.getResource("/x1.txt")
  def name: String = getClass.getName
