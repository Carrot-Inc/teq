// jars: scala-library zio-json magnolia zio
//> using dep dev.zio::zio-json:0.9.2
// targets: js interp jvm
// zio-json's derived decoder of a case class of more than 22 fields: Magnolia's `rawConstruct`
// collects the decoded fields in an array, `Tuple.fromArray` makes a tuple of 34 elements
// (scala-library's TupleXXL, the lean std's on JavaScript and the interpreter) and the mirror's
// `fromProduct` reads it back as the class. Decoding, a field's error, a round trip.
import zio.json.*

case class Hold(
    id: Int, name: String, placed: Long, paid: Boolean, total: Double, note: Option[String],
    branch: Int, city: String, slot: Long, ready: Boolean, weight: Double, door: Option[String],
    items: Int, phone: String, eta: Long, fragile: Boolean, fine: Double, waiver: Option[String],
    lane: Int, clerk: String, updated: Long, cold: Boolean, tax: Double, gate: Option[String],
    bags: Int, zone: String, created: Long, gift: Boolean, fee: Double, floor: Option[String],
    count: Int, region: String, version: Long, archived: Boolean) derives JsonCodec

case class Response(holds: List[Hold], total: Int) derives JsonDecoder

object Main:
  def main(args: Array[String]): Unit =
    val one = """{"id":7,"name":"Ada","placed":10000000001,"paid":true,"total":12.5,"branch":3,"city":"Oslo",""" +
      """"slot":10000000002,"ready":false,"weight":1.25,"door":"B","items":4,"phone":"555","eta":10000000003,""" +
      """"fragile":true,"fine":2.25,"lane":9,"clerk":"Bo","updated":10000000004,"cold":true,"tax":0.5,"gate":"G2",""" +
      """"bags":2,"zone":"N","created":10000000005,"gift":false,"fee":1.5,"floor":"3","count":11,"region":"EU",""" +
      """"version":10000000006,"archived":false}"""
    val decoded = one.fromJson[Hold]
    println(decoded)
    println(decoded.map(_.region))
    println(s"""{"holds":[$one,$one],"total":2}""".fromJson[Response].map(r => r.holds.size.toString + " " + r.total))
    println(one.replace("\"lane\":9", "\"lane\":\"nine\"").fromJson[Hold])
    println(decoded.map(p => p.toJson.fromJson[Hold] == Right(p)))
