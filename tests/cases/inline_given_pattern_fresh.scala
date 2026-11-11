import scala.compiletime.summonFrom

inline def repeated(using value: String): String =
  summonFrom {
    case given String => summon[String] + summon[String]
  }

inline def nested(using value: String): String =
  summonFrom {
    case given String =>
      summonFrom {
        case given String => repeated + summon[String]
      }
  }

@main def Test(): Unit =
  given String = "x"
  println(repeated)
  println(nested)
