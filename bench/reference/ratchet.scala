//> using file ../../tools/script
//> using file records.scala
// The reference projects' records held to the ratchet (bench/reference/records.scala; docs/DEVELOPING.md, "The
// project records"). Run from the repository's root:
//
//   ./teq interp bench/reference/ratchet.scala -- <verb> ...
//
// check <revision>          every record of the projects under bench/ (errors.txt, deviations.json, run.json), the
//                           revision's and the head's alike, against the revision's committed one, the accepted
//                           record, independently of any run: the gate's line `records`, against master as it is; a
//                           head that does not contain the revision is refused, and a record of the revision's the head
//                           lacks, its project's directory there or not. A line per record and a summary; exit 1 when
//                           one is refused.
// compile <project> <transcript> <cwd> <checkout> <out> [--record | --baseline | --migrate <reason>] [--binary <version>]
//                           a compile line's transcript (ended by its verb's end marker; its builds' diagnostics as
//                           teq recorded them in <transcript>.diagnostics, `--diagnostics`) against the project's
//                           errors.txt, as the compile verbs hold a run (`Lines.hold`): the roots of the directory teq
//                           ran in, the checkout and the logs' directory normalised; exit 1 when the run is not within
//                           it; with a mode, the record's step as the verbs take it, the binary's version line its
//                           provenance.
// install <project> <file> <proposal>
//                           a proposal a project line's --record staged (the gate's collection) installed as the
//                           project's record of that file, admitted against the record it replaces first (a step of
//                           the record: a baseline and a migration are the verbs', with their evidence).
import java.nio.file.{Files, Path, Paths}

@main def ratchet(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/reference/ratchet.scala <check <revision> | compile <project> <transcript> <cwd> <checkout> <out> [--record | --baseline | --migrate <reason>] [--binary <version>] | install <project> <file> <proposal>>")
  val mode = Mode.of(args)
  val binary = args.option("binary").getOrElse("unknown")
  val positionals = args.positionals
  if !Files.isDirectory(Paths.get("bench/reference")) then Log.die("bench/reference/ratchet.scala runs from the repository's root")
  positionals match
    case Vector("check", revision) => Script.exit(if Ratchets.check(revision) then 0 else 1)
    case Vector("compile", project, transcript, cwd, checkout, out) =>
      val here = Paths.get(project)
      val roots = Roots.of(Paths.get(cwd), Paths.get(checkout), Paths.get(out))
      val text = Files.readString(Paths.get(transcript))
      val records = Transcript.recorded(Paths.get(transcript + ".diagnostics"))
      val passed = Lines.hold(here.getFileName.toString, "compile", here, Shape.Compile, Vector("compile" -> Transcript.compile(text, records, roots)), roots(text), mode, binary)
      Script.exit(if passed then 0 else 1)
    case Vector("install", project, file, proposal) =>
      val here = Paths.get(project)
      val shape = Shape.all.find(_.file == file).getOrElse(args.fail(s"no record named $file (${Shape.all.map(_.file).mkString(", ")})"))
      val proposed = Records.parse(shape, Files.readString(Paths.get(proposal))).fold(why => Log.die(s"$proposal does not read: $why"), identity)
      // The gate's proposals are records' steps, whose evidence is the baseline's or the migration's already there.
      val problems = Records.publish(here, shape, proposed, None)
      for p <- problems do println(if p.startsWith("  ") then p else s"FAIL records: $here/$file: the proposal is refused: $p")
      if problems.isEmpty then println(s"records: $here/$file: ${shape.counted(proposed.debt.size)} installed (${proposed.provenance.kind})")
      Script.exit(if problems.isEmpty then 0 else 1)
    case _ => args.fail()
}

object Ratchets:
  // Every record of every project against the revision's: whether all were admitted.
  def check(revision: String): Boolean =
    val rev = Sh("git", "rev-parse", "--verify", "-q", s"$revision^{commit}").timeout(60).check(false).run()
    if !rev.ok then Log.die(s"records: $revision is no commit of this repository")
    // The accepted records are the revision's, master's as it is now: a head that does not contain it would be held to
    // an older floor than master's, and is refused until it is rebased.
    if !Sh("git", "merge-base", "--is-ancestor", rev.out.trim, "HEAD").timeout(60).check(false).run().ok then
      println(s"FAIL records: the head does not contain $revision (${rev.out.trim.take(9)}), whose records are the accepted ones: rebase onto it")
      return false
    // The records of either side: master's committed ones and the head's, whatever holds a pin, so that a floor goes
    // with neither its record nor its project's directory.
    val names = Shape.all.map(_.file).toSet
    def recordPath(p: String) = p.split('/') match
      case Array("bench", _, file) if names.contains(file) => true
      case _ => false
    val committed = Sh("git", "ls-tree", "-r", "--name-only", rev.out.trim, "--", "bench").timeout(60).run().out.linesIterator.filter(recordPath).toVector
    val present = Files.list(Paths.get("bench")).toArray.toVector.map(_.asInstanceOf[Path]).filter(Files.isDirectory(_))
      .flatMap(d => Shape.all.map(sh => d.resolve(sh.file))).filter(Files.isRegularFile(_)).map(_.toString)
    var records = 0
    var refused = 0
    for path <- (committed ++ present).distinct.sorted do
      val here = Paths.get(path).getParent
      val shape = Shape.all.find(_.file == Paths.get(path).getFileName.toString).get
      val shown = Sh("git", "show", s"${rev.out.trim}:$path").timeout(60).check(false).run()
      val accepted = Option.when(shown.ok)(shown.outAll()).map(text => Records.parse(shape, text).fold(why => Log.die(s"records: $path at $revision does not read: $why"), identity))
      val candidate = Records.read(here, shape)
      if accepted.nonEmpty || candidate.nonEmpty then
        records += 1
        val problems = Ratchet.admit(accepted, candidate, Records.inputsOf(here, shape), Files.isRegularFile(here.resolve(shape.evidence)))
        if problems.nonEmpty then
          refused += 1
          problems.foreach(p => println(if p.startsWith("  ") then p else s"FAIL records: $path: $p"))
        else
          val c = candidate.get
          val n = c.debt.size
          val against = accepted match
            case None => s"a ${c.provenance.kind}, master has none"
            case Some(a) if a == c => "as accepted"
            case Some(a) if a.provenance.inputs != c.provenance.inputs => s"a migration (${c.provenance.reason}) from master's ${a.debt.size}"
            case Some(a) => s"${a.debt.size - n} fewer than master's ${a.debt.size}"
          println(s"records: $path: ${shape.counted(n)}, $against")
    println(s"records: $records records against $revision, $refused refused")
    refused == 0
