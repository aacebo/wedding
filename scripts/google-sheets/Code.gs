const HEADERS = [
  "RSVP ID",
  "Submitted At (UTC)",
  "First Name",
  "Last Name",
  "Guest Name (Legacy/Full)",
  "Email",
  "Attendance",
  "Accommodation Name",
  "Accommodation Address",
  "Phone (with country code)",
  "Dietary / Allergies / Intolerances",
  "Main Course Choice",
  "Legacy Party Size"
];

function doPost(e) {
  const properties = PropertiesService.getScriptProperties();
  const expectedSecret = properties.getProperty("SHARED_SECRET");
  const spreadsheetId = properties.getProperty("SPREADSHEET_ID");
  const sheetName = properties.getProperty("SHEET_NAME") || "RSVPs";

  try {
    if (!expectedSecret || !spreadsheetId || !e || !e.postData || !e.postData.contents) {
      return json({ ok: false, error: "Webhook is not configured." });
    }

    const request = JSON.parse(e.postData.contents);
    if (!constantTimeEqual(String(request.secret || ""), expectedSecret)) {
      return json({ ok: false, error: "Unauthorized." });
    }
    if (!request.rsvp || !request.rsvp.rsvp_id) {
      return json({ ok: false, error: "Missing RSVP ID." });
    }

    const lock = LockService.getScriptLock();
    lock.waitLock(20000);
    try {
      const sheet = SpreadsheetApp.openById(spreadsheetId).getSheetByName(sheetName);
      if (!sheet) throw new Error("Sheet tab not found: " + sheetName);
      const lastRow = sheet.getLastRow();
      if (lastRow === 0) sheet.getRange(1, 1, 1, HEADERS.length).setValues([HEADERS]);

      const rsvp = request.rsvp;
      const row = [
        String(rsvp.rsvp_id),
        safeText(rsvp.submitted_at),
        safeText(rsvp.first_name),
        safeText(rsvp.last_name),
        safeText(rsvp.guest_name),
        safeText(rsvp.email),
        safeText(rsvp.attendance),
        safeText(rsvp.accommodation_name),
        safeText(rsvp.accommodation_address),
        safeText(rsvp.phone),
        safeText(rsvp.dietary_requirements),
        safeText(rsvp.main_course_choice),
        rsvp.legacy_party_size == null ? "" : Number(rsvp.legacy_party_size)
      ];

      const currentLastRow = sheet.getLastRow();
      const ids = currentLastRow > 1
        ? sheet.getRange(2, 1, currentLastRow - 1, 1).getDisplayValues().flat()
        : [];
      const index = ids.indexOf(String(rsvp.rsvp_id));
      const targetRow = index >= 0 ? index + 2 : Math.max(2, currentLastRow + 1);
      sheet.getRange(targetRow, 1, 1, HEADERS.length).setValues([row]);
      return json({ ok: true, row: targetRow });
    } finally {
      lock.releaseLock();
    }
  } catch (error) {
    return json({ ok: false, error: String(error && error.message || error) });
  }
}

function safeText(value) {
  if (value == null) return "";
  const text = String(value);
  return /^[=+\-@]/.test(text) ? "'" + text : text;
}

function constantTimeEqual(a, b) {
  if (a.length !== b.length) return false;
  let difference = 0;
  for (let i = 0; i < a.length; i++) {
    difference |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return difference === 0;
}

function json(value) {
  return ContentService.createTextOutput(JSON.stringify(value))
    .setMimeType(ContentService.MimeType.JSON);
}
