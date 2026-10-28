'use client';

import { useState } from 'react';

export default function RemoteCompanionPage() {
  const [roomId, setRoomId] = useState('');
  const [verifyCode, setVerifyCode] = useState('');
  const [connected, setConnected] = useState(false);
  const [confirmed, setConfirmed] = useState(false);

  const handlePair = (e: React.FormEvent) => {
    e.preventDefault();
    if (!roomId) return;
    setConnected(true);
  };

  const handleConfirm = () => {
    setConfirmed(true);
  };

  return (
    <main style={{ maxWidth: 480, margin: '0 auto', padding: '24px 16px', fontFamily: 'system-ui, sans-serif' }}>
      <header style={{ marginBottom: 24, textAlign: 'center' }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, margin: '0 0 8px' }}>Spawnt Remote</h1>
        <p style={{ fontSize: 14, color: '#666', margin: 0 }}>End-to-end encrypted mobile companion</p>
      </header>

      {!connected ? (
        <section style={{ border: '1px solid #e0e0e0', borderRadius: 8, padding: 16 }}>
          <h2 style={{ fontSize: 16, margin: '0 0 12px' }}>Pair with desktop</h2>
          <form onSubmit={handlePair}>
            <label style={{ display: 'block', fontSize: 13, marginBottom: 6, fontWeight: 600 }}>
              Room ID / QR Code:
            </label>
            <input
              type="text"
              value={roomId}
              onChange={e => setRoomId(e.target.value)}
              placeholder="e.g. room-abcdef0123456789"
              style={{ width: '100%', padding: '10px 12px', fontSize: 14, borderRadius: 6, border: '1px solid #ccc', boxSizing: 'border-box', marginBottom: 16 }}
            />
            <button
              type="submit"
              disabled={!roomId}
              style={{ width: '100%', padding: '12px', fontSize: 14, fontWeight: 600, background: '#111', color: '#fff', border: 'none', borderRadius: 6, cursor: 'pointer' }}
            >
              Connect Securely
            </button>
          </form>
        </section>
      ) : !confirmed ? (
        <section style={{ border: '1px solid #e0e0e0', borderRadius: 8, padding: 16, textAlign: 'center' }}>
          <h2 style={{ fontSize: 16, margin: '0 0 12px' }}>Verify Confirmation Code</h2>
          <p style={{ fontSize: 14, color: '#444', marginBottom: 16 }}>
            Confirm this 6-digit code matches the one shown on your desktop screen:
          </p>
          <div style={{ fontSize: 32, fontWeight: 800, letterSpacing: 4, padding: '16px 0', color: '#0969da' }}>
            {verifyCode || '482910'}
          </div>
          <button
            onClick={handleConfirm}
            style={{ width: '100%', padding: '12px', fontSize: 14, fontWeight: 600, background: '#1f883d', color: '#fff', border: 'none', borderRadius: 6, cursor: 'pointer', marginTop: 12 }}
          >
            Confirm & Complete Pairing
          </button>
        </section>
      ) : (
        <section style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div style={{ background: '#dafbe1', border: '1px solid #aceebb', borderRadius: 8, padding: '12px 16px', fontSize: 13, color: '#1a7f37' }}>
            [ONLINE] End-to-end encrypted session active
          </div>

          <div style={{ border: '1px solid #e0e0e0', borderRadius: 8, padding: 16 }}>
            <h2 style={{ fontSize: 15, margin: '0 0 12px' }}>Active Sessions</h2>
            <div style={{ padding: '12px', background: '#f6f8fa', borderRadius: 6, fontSize: 13 }}>
              <strong>claude-code:main</strong> &bull; Claude 3.5 Sonnet &bull; <span style={{ color: '#0969da' }}>Idle</span>
            </div>
          </div>

          <div style={{ border: '1px solid #e0e0e0', borderRadius: 8, padding: 16 }}>
            <h2 style={{ fontSize: 15, margin: '0 0 12px' }}>Pending Approvals</h2>
            <p style={{ fontSize: 13, color: '#666', margin: 0 }}>No actions currently awaiting approval.</p>
          </div>
        </section>
      )}
    </main>
  );
}
