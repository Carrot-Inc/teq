// jars: tapir-core sttp-model sttp-shared-core sttp-shared-ws magnolia123 scala-java-time
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
// tapir's derivation `Configuration` from its jar without scala-library on the class path:
// `Configuration.default` names `Predef.identity`, which is the std's, and the other presets
// transform field and subtype names.
import sttp.tapir.generic.Configuration

object Main:
  def main(args: Array[String]): Unit =
    val d = Configuration.default
    println(d.toEncodedName("petName") + " " + d.discriminator)
    println(d.withSnakeCaseMemberNames.toEncodedName("petName") + " " + d.withKebabCaseMemberNames.toEncodedName("petName"))
    println(d.withScreamingSnakeCaseMemberNames.toEncodedName("petName") + " " + d.withDiscriminator("kind").discriminator)
