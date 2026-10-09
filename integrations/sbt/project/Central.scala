import sbt.*
import sbt.Keys.*
import com.jsuereth.sbtpgp.PgpKeys.pgpSelectPassphrase

/** What the plugin's build takes for Maven Central, where the plugin alone is published (the compiler's binaries go to
  * the GitHub release): the pom Central requires, the javadoc jar it requires beside the jar (a README), and sbt-pgp's
  * signing. sbt stages and signs a release into `localStaging`; it never
  * uploads one: central.py does, to the Central Portal as a deployment it promotes or drops by its id. So the build
  * loads no Portal token, and drops sbt's `sonaUpload` and `sonaRelease`, which bundle whatever the staging directory
  * holds and upload it anew each time, `sonaRelease` to be published automatically: one mistyped command would
  * publish what nobody checked, and Central never takes it back. */
object Central {
  val repository = "https://github.com/Carrot-Inc/teq"

  def settings: Seq[Setting[?]] = Seq(
    homepage := Some(uri("https://teq.build")),
    licenses := List(License.Apache2),
    developers := List(Developer("carrot", "Carrot", "", uri(repository))),
    scmInfo := Some(ScmInfo(uri(repository), s"scm:git:$repository.git")),
    publishMavenStyle := true,
    pomIncludeRepository := (_ => false),
    publishTo := localStaging.value,
    // The key is the caller's (publish.sh sets the release's); sbt-pgp signs through gpg and its agent,
    // and never with a passphrase on gpg's command line, which it would print whole when gpg fails: the
    // release's key has none, and a key that wants one fails the signature, gpg having no terminal to ask on.
    pgpSelectPassphrase := None,
    Global / commands ~= (_.filterNot(c => c.nameOption.exists(Set("sonaUpload", "sonaRelease")))),
  )

  /** The jar Central requires beside the main artifact (`sources` or `javadoc`), holding a README alone: what
    * the module is and where its sources are, at the commit the release is built from. */
  def placeholder(what: String, text: Def.Initialize[String]): Def.Initialize[Task[Seq[(xsbti.HashedVirtualFileRef, String)]]] = Def.task {
    val converter = fileConverter.value
    val file = target.value / "central" / what / "README.md"
    IO.write(file, text.value)
    Seq(converter.toVirtualFile(file.toPath) -> "README.md")
  }

  /** The commit the build's checkout is at, which the placeholders name. */
  def commit(base: File): String =
    scala.util.Try(scala.sys.process.Process(Seq("git", "rev-parse", "HEAD"), base).!!.trim).getOrElse("")
}
