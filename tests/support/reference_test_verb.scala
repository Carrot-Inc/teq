//> using file ../../tools/script
//> using file ../../bench/reference/reference.scala
//> using file ../../bench/reference/scalajs.scala
//> using file ../../bench/reference/records.scala
// The test verb of the Scala.js reference projects held to an inventory (tests/reference-test-verb.sh): Airstream's
// `ScalaJsReference.test` over the inventory <inventory>, a checkout <dir> and the stand-in teq <bin>, whose
// transcript and events the verb judges, in a directory of the check's own under <out> with the pin and the inventory.
// Without a fifth argument its departures are held to a deviations.json of none (`Records.publish`, a baseline of no
// departure: a run as the inventory's keeps it); with `baseline` the run's --baseline over the directory made afresh,
// with `kept` its judgment and with `record` its --record over the record the directory holds.
import java.nio.file.{Files, Paths, StandardCopyOption}

@main def referenceTestVerb(argv: String*): Unit = Script.run {
  val Seq(inventory, dir, out, bin) = argv.take(4): @unchecked
  val how = argv.lift(4).getOrElse("")
  val here = Paths.get(out).resolve("here")
  if how == "" || how == "baseline" then
    Sh.remove(here)
    Files.createDirectories(here)
    Files.copy(Paths.get("bench/airstream/pin.json"), here.resolve("pin.json"))
    if Files.isRegularFile(Paths.get(inventory)) then Files.copy(Paths.get(inventory), here.resolve("tests.json"), StandardCopyOption.REPLACE_EXISTING)
  if how == "" && Files.isRegularFile(here.resolve("tests.json")) then
    val projects = Tests.read(here.resolve("tests.json")).heldSuites.map(_._1).distinct.sorted
    val none = Record(Lines.provenance("baseline", "", here, Shape.Tests, "teq of the check", Map.empty), projects, Vector.empty)
    for p <- Records.publish(here, Shape.Tests, none, Some("a record of no departure, the check's")) do Log.die(s"no record of none: $p")
  val mode = how match
    case "baseline" => Mode.Baseline
    case "record" => Mode.Record
    case _ => Mode.Judge
  new ScalaJsReference("airstream", here).test(Paths.get(dir), Paths.get(out), bin, 1, mode, here.resolve("tests.json"))
}
