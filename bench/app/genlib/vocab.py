"""The corpus's own vocabulary: an observatory network that schedules telescopes, instruments,
observers and their nights. Names are composed from these words deterministically."""

NOUNS = [
    "telescope", "dome", "mount", "instrument", "detector", "filter", "target", "proposal", "observer",
    "operator", "exposure", "calibration", "weather", "seeing", "night", "twilight", "sequence",
    "queue", "schedule", "slot", "window", "station", "archive", "frame", "pointing", "tracking",
    "focus", "aperture", "mirror", "camera", "spectrograph", "grating", "catalog", "ephemeris", "transit",
    "eclipse", "occultation", "comet", "asteroid", "nebula", "cluster", "galaxy", "quasar", "pulsar", "orbit",
    "epoch", "coordinate", "declination", "ascension", "altitude", "azimuth", "magnitude", "flux", "spectrum",
    "wavelength", "band", "channel", "gain", "offset", "bias", "flat", "pipeline", "reduction", "alert",
    "ticket", "maintenance", "visit", "booking", "visitor", "outreach", "grant", "budget", "invoice", "member",
    "team", "shift", "roster", "dark", "lamp", "shutter", "cooler", "cryostat", "vacuum", "readout", "binning",
    "guider", "autoguider", "sensor", "anemometer", "hygrometer", "rain", "cloud", "dust", "moon", "sun", "horizon",
    "zenith", "pier", "enclosure", "vent", "louver", "beacon", "relay", "uplink", "downlink", "packet",
    "manifest", "checklist", "handover", "logbook", "note", "comment", "review", "verdict", "score", "rank",
    "priority", "deadline", "cycle", "semester", "quarter", "season", "forecast", "outlook", "trend", "baseline",
    "threshold", "limit", "quota", "credit", "tariff", "rate", "fee", "refund", "ledger", "account", "profile",
    "preference", "setting", "theme", "layout", "widget", "panel", "drawer", "toolbar", "badge", "chip", "toast",
    "banner", "modal", "wizard", "stepper", "tab", "card", "table", "chart", "gauge", "sparkline", "timeline",
    "overlay", "marker", "leg", "waypoint", "vehicle", "courier", "shipment", "crate", "part",
    "spare", "tool", "kit", "supplier", "order", "quote", "contract", "permit", "license", "certificate", "audit",
    "incident", "hazard", "drill", "evacuation", "power", "generator", "battery", "grid", "outage", "load",
]

QUALIFIERS = [
    "nightly", "pending", "archived", "calibrated", "primary", "secondary", "remote", "manual", "automatic",
    "draft", "active", "retired", "planned", "actual", "expected", "measured", "raw", "reduced", "public",
    "private", "internal", "external", "urgent", "deferred", "partial", "final", "initial", "current", "previous",
    "next", "annual", "monthly", "weekly", "daily", "hourly", "peak", "mean", "total", "net", "gross", "local",
    "global", "north", "south", "east", "west", "upper", "lower", "inner", "outer", "narrow", "wide", "short",
    "long", "bright", "faint", "warm", "cold", "wet", "dry", "clear", "hazy", "stable", "unstable",
]

VERBS = [
    "schedule", "allocate", "calibrate", "expose", "archive", "review", "approve", "reject", "cancel", "resume",
    "publish", "retract", "assign", "release", "reserve", "confirm", "notify", "escalate", "resolve", "reopen",
    "merge", "split", "rank", "score", "estimate", "forecast", "measure", "record", "replay", "verify", "audit",
    "rotate", "park", "unpark", "slew", "track", "guide", "focus", "cool", "warm", "vent", "seal", "open", "close",
]

UNITS = ["seconds", "minutes", "hours", "days", "millimetres", "metres", "degrees", "arcsec", "nanometres", "kelvin", "percent", "count", "index", "level", "code", "kind", "mode", "state", "status", "phase", "stage"]

CASE_WORDS = [
    "Idle", "Ready", "Busy", "Parked", "Slewing", "Tracking", "Cooling", "Warming", "Open", "Closed", "Sealed",
    "Draft", "Submitted", "Accepted", "Rejected", "Scheduled", "Running", "Paused", "Done", "Failed", "Cancelled",
    "Queued", "Blocked", "Expired", "Archived", "Pending", "Approved", "Declined", "Deferred", "Escalated",
    "Clear", "Hazy", "Overcast", "Rain", "Snow", "Fog", "Windy", "Calm", "Humid", "Dry", "Dusty",
    "New", "Waxing", "Full", "Waning", "Rising", "Setting", "Transit", "Twilight", "Dawn", "Dusk", "Midnight",
    "Optical", "Infrared", "Radio", "Ultraviolet", "Xray", "Gamma", "Wide", "Narrow", "Broad", "Blue", "Red",
    "Green", "Amber", "Violet", "Clearband", "Halpha", "Oiii", "Sii", "Luminance", "Bias", "Dark", "Flat",
    "Primary", "Secondary", "Tertiary", "North", "South", "East", "West", "Zenith", "Horizon", "Manual",
    "Automatic", "Remote", "Local", "Guest", "Member", "Staff", "Admin", "Owner", "Viewer", "Editor", "Reviewer",
    "Low", "Medium", "High", "Critical", "Info", "Warning", "Error", "Fatal", "Trace", "Debug",
]

FIRST_NAMES = ["Ada", "Bram", "Cleo", "Dov", "Eris", "Faye", "Gil", "Hana", "Ivo", "Juno", "Kai", "Lior", "Mira", "Nils", "Ora", "Pim", "Quin", "Rhea", "Sol", "Tove", "Uma", "Vik", "Wren", "Xia", "Yael", "Zed"]
PLACE_NAMES = ["Ridgecrest", "Hollowmere", "Stonevale", "Ashford", "Brightwater", "Coldharbour", "Dunmoor", "Eastreach", "Fairhollow", "Greywick", "Highcairn", "Ironpeak"]


def camel(words):
    return "".join(w[0].upper() + w[1:] for w in words)


def lower_camel(words):
    head = camel(words)
    return head[0].lower() + head[1:]
