// An export through an earlier clause's forwarder of a stable member, itself a path: dotty gives the
// forwarder `StableRealizable` (`Namer.exportForwarders`), so `tournamentId.isDefined` is exportable.
case class Metadata(tournamentId: Option[String], simulId: Option[String])
class Game(val metadata: Metadata):
  export metadata.{ tournamentId, simulId }
  export tournamentId.isDefined as isTournament
  export simulId.isDefined as isSimul
  def isMandatory = isTournament || isSimul
@main def main() = println((Game(Metadata(Some("t"), None)).isMandatory, Game(Metadata(None, None)).isTournament))
