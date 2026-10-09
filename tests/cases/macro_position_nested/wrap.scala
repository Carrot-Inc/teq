object Wrap:
  inline def wrapper: String = Pos.here
  inline def twice: String = wrapper + " " + Pos.here
  inline def genWrapped: String = Pos.gen
  inline def fileWrapped: String = Pos.file
