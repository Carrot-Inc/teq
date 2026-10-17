//> using file ../../bench/mill-reference.scala
// bench/mill-reference.scala's test verb held to an inventory (tests/mill-test-verb.sh): `Ref.test` with the
// pin and the inventory in <here>, a checkout <dir> holding the export's lock and the stand-in teq <bin>, whose
// listing and transcript the verb compares. Without a fifth argument its departures are held to a deviations.json of
// none over the inventory's modules, which the check makes in <here> (`Records.publish`, a baseline of no departure: a
// run as the inventory's keeps it); with `record`, `baseline` or `kept`, the run's --record, its --baseline (the record
// removed first) or its judgment over the record <here> holds.
import java.nio.file.{Files, Paths}

@main def millTestVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin) = argv.take(4): @unchecked
  val how = argv.lift(4).getOrElse("")
  val h = Paths.get(here)
  if how == "" || how == "baseline" then Files.deleteIfExists(h.resolve("deviations.json"))
  if how == "" && Files.isRegularFile(h.resolve("tests.json")) then
    val modules = Inventory.read(h.resolve("tests.json")).modules.keys.toVector.sorted
    val none = Record(Lines.provenance("baseline", "", h, Shape.Tests, "teq of the check", Map.empty), modules, Vector.empty)
    for p <- Records.publish(h, Shape.Tests, none, Some("a record of no departure, the check's")) do Log.die(s"no record of none: $p")
  val mode = how match
    case "record" => Mode.Record
    case "baseline" => Mode.Baseline
    case _ => Mode.Judge
  Ref.test(Pin.read("tyrian", h), Paths.get(dir), Paths.get(out), bin, 1, mode)
}
