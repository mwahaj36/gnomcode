-- Apply once in the SAME Supabase project as the web app and collectors.
-- Only encrypted, short-lived login envelopes are retained. No plaintext credentials.
BEGIN;
CREATE TABLE IF NOT EXISTS public.fleethud_pairing_envelopes (
    request_id text PRIMARY KEY CHECK (request_id ~ '^[0-9a-f]{32}$'),
    user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    poll_hash text NOT NULL CHECK (poll_hash ~ '^[0-9a-f]{64}$'),
    ciphertext text CHECK (length(ciphertext) BETWEEN 24 AND 90000),
    iv text CHECK (length(iv) = 16),
    expires_at timestamptz NOT NULL DEFAULT now() + interval '5 minutes'
);
ALTER TABLE public.fleethud_pairing_envelopes ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON public.fleethud_pairing_envelopes FROM PUBLIC, anon, authenticated;

CREATE OR REPLACE FUNCTION public.fleethud_pair_approve(
    p_request_id text, p_poll_hash text, p_ciphertext text, p_iv text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path = '' AS $$
BEGIN
    IF (SELECT auth.uid()) IS NULL THEN RAISE EXCEPTION 'Sign in to approve pairing'; END IF;
    DELETE FROM public.fleethud_pairing_envelopes WHERE expires_at <= now();
    -- Serialize approvals per account so concurrent requests cannot bypass the limit.
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended((SELECT auth.uid())::text, 0));
    IF (SELECT count(*) FROM public.fleethud_pairing_envelopes WHERE user_id = (SELECT auth.uid())) >= 10
        THEN RAISE EXCEPTION 'Too many pairing requests; wait five minutes'; END IF;
    IF p_ciphertext IS NULL OR p_iv IS NULL OR p_poll_hash IS NULL OR p_request_id IS NULL
        THEN RAISE EXCEPTION 'Incomplete pairing envelope'; END IF;
    INSERT INTO public.fleethud_pairing_envelopes(request_id,user_id,poll_hash,ciphertext,iv)
        VALUES(p_request_id,(SELECT auth.uid()),p_poll_hash,p_ciphertext,p_iv);
END;
$$;

CREATE OR REPLACE FUNCTION public.fleethud_pair_consume(p_request_id text, p_poll_token text)
RETURNS jsonb LANGUAGE plpgsql SECURITY DEFINER SET search_path = '' AS $$
DECLARE envelope jsonb;
BEGIN
    IF length(p_poll_token) <> 64 OR p_poll_token !~ '^[0-9a-f]{64}$' THEN RETURN NULL; END IF;
    SELECT jsonb_build_object('ciphertext',ciphertext,'iv',iv) INTO envelope
        FROM public.fleethud_pairing_envelopes
        WHERE request_id = p_request_id AND expires_at > now() AND ciphertext IS NOT NULL
        AND poll_hash = encode(sha256(convert_to(p_poll_token,'UTF8')),'hex') FOR UPDATE;
    IF envelope IS NOT NULL THEN
        -- Keep a tombstone until expiry: the same request cannot be approved twice.
        UPDATE public.fleethud_pairing_envelopes SET ciphertext = NULL, iv = NULL WHERE request_id = p_request_id;
    END IF;
    RETURN envelope;
END;
$$;
REVOKE ALL ON FUNCTION public.fleethud_pair_approve(text,text,text,text) FROM PUBLIC, anon;
GRANT EXECUTE ON FUNCTION public.fleethud_pair_approve(text,text,text,text) TO authenticated;
REVOKE ALL ON FUNCTION public.fleethud_pair_consume(text,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.fleethud_pair_consume(text,text) TO anon, authenticated;
COMMIT;
