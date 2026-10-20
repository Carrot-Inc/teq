object Model:
  final case class Code(value: String) derives Show

  object Code:
    def parse(s: String): Option[Code] = if s.nonEmpty && s.forall(_.isUpper) then Some(Code(s)) else None

  final case class Name(value: String) derives Show
