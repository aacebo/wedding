DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM rsvps GROUP BY guest_id HAVING COUNT(*) > 1) THEN
        RAISE EXCEPTION 'Multiple RSVPs exist for a guest. Reconcile these responses before retrying migration 003; no responses have been deleted.';
    END IF;
    IF EXISTS (SELECT 1 FROM guests GROUP BY lower(btrim(email)) HAVING COUNT(*) > 1) THEN
        RAISE EXCEPTION 'Guest emails collide after trimming and lowercasing. Reconcile these guest identities before retrying migration 003; no guests have been merged.';
    END IF;
END $$;

ALTER TABLE rsvps ADD COLUMN party_size SMALLINT;
UPDATE rsvps SET party_size = CASE WHEN attending THEN 1 ELSE 0 END;
ALTER TABLE rsvps ALTER COLUMN party_size SET NOT NULL;
ALTER TABLE rsvps ADD CONSTRAINT rsvps_party_size_check
    CHECK ((attending AND party_size BETWEEN 1 AND 2) OR (NOT attending AND party_size = 0));
ALTER TABLE rsvps ADD CONSTRAINT rsvps_guest_id_key UNIQUE (guest_id);

CREATE UNIQUE INDEX guests_normalized_email_key ON guests (lower(btrim(email)));
