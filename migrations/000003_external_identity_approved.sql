-- Add approved column to external_identity for admin-approve provisioning policy
-- OIDC identities are created with approved=0 and require admin approval before active session
ALTER TABLE external_identity ADD COLUMN approved INTEGER NOT NULL DEFAULT 0;
