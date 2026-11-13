object R2:
  def resource: AnyRef = getClass.getResource("/x2.txt")
  def name: String = getClass.getName
