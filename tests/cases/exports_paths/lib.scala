package lib

object internal:
  object Icons:
    case class Icon(name: String)
    def home: Icon = Icon("home")
  object Colors:
    val red: String = "#f00"

// the second clause starts with a name that the first one exports
export internal.Icons
export Icons.{Icon, home as homeIcon}
