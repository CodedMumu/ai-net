/**
 * AgentRegistryCard
 *
 * Displays a single registered agent from the backend registry in card format.
 * Shows: name (large, bold) + status dot, capability badges (color-coded),
 * price in XLM + USD estimate, reputation stars, and a "Hire" CTA that
 * appears on card hover.
 *
 * Redesigned for improved scannability (Issue #97).
 */

import React from 'react';
import { useTranslation } from 'react-i18next';
import { Star, Zap } from 'lucide-react';
import type { AgentRecord } from '../../types/api';
import styles from './AgentRegistryCard.module.css';

export interface AgentRegistryCardProps {
  agent: AgentRecord;
  onHire?: (agent: AgentRecord) => void;
}

/** Map capability names to color keys for pill styling. */
const CAPABILITY_COLORS: Record<string, string> = {
  research: 'blue',
  risk: 'orange',
  coding: 'green',
  design: 'purple',
  report: 'red',
  audit: 'orange',
};

function ReputationStars({ value }: { value: number }) {
  const rounded = Math.round(Math.max(0, Math.min(5, value)));
  return (
    <span className={styles.stars} aria-label={`Reputation: ${value.toFixed(1)} out of 5`}>
      {Array.from({ length: 5 }, (_, i) => (
        <Star
          key={i}
          size={12}
          className={i < rounded ? styles.starFilled : styles.starEmpty}
          aria-hidden="true"
        />
      ))}
      <span className={styles.reputationValue}>{value.toFixed(1)}</span>
    </span>
  );
}

function CapabilityPill({ capability }: { capability: string }) {
  const colorKey = CAPABILITY_COLORS[capability.toLowerCase()] ?? 'default';
  return (
    <span className={`${styles.pill} ${styles[`pill--${colorKey}`]}`}>
      {capability}
    </span>
  );
}

export const AgentRegistryCard: React.FC<AgentRegistryCardProps> = ({ agent, onHire }) => {
  const { t } = useTranslation();
  const isOnline = agent.status === 'active';

  const handleHire = (e: React.MouseEvent) => {
    e.stopPropagation();
    onHire?.(agent);
  };

  const usdEstimate = (agent.price * 0.1).toFixed(2);

  return (
    <article
      className={styles.card}
      data-testid={`agent-card-${agent.id}`}
      aria-label={`${agent.name} agent card`}
    >
      {/* ── Header: avatar + name + status dot ───────────────────── */}
      <div className={styles.header}>
        <div className={styles.avatar} aria-hidden="true">
          <Zap size={18} />
        </div>

        <div className={styles.nameBlock}>
          <div className={styles.nameRow}>
            <h3 className={styles.name}>{agent.name}</h3>
            {/* Status dot — subtle, next to name, no large badge */}
            <span
              className={`${styles.statusDot} ${isOnline ? styles.statusDotOnline : styles.statusDotOffline}`}
              aria-label={isOnline ? 'Online' : 'Offline'}
              data-testid={`agent-status-${agent.id}`}
              title={isOnline
                ? t('agent.status.active', { defaultValue: 'Online' })
                : t('agent.status.inactive', { defaultValue: 'Offline' })}
            />
          </div>
          <code className={styles.agentId} title={agent.id}>
            {agent.id.length > 16
              ? `${agent.id.slice(0, 10)}…${agent.id.slice(-4)}`
              : agent.id}
          </code>
        </div>
      </div>

      {/* ── Capability badges ────────────────────────────────────── */}
      <div className={styles.capabilities} data-testid={`agent-capabilities-${agent.id}`}>
        {agent.capabilities.length === 0 ? (
          <span className={styles.noCaps}>
            {t('common.none', { defaultValue: 'None' })}
          </span>
        ) : (
          agent.capabilities.map((cap) => (
            <CapabilityPill key={cap} capability={cap} />
          ))
        )}
      </div>

      {/* ── Price + Reputation ───────────────────────────────────── */}
      <div className={styles.priceBlock} data-testid={`agent-price-${agent.id}`}>
        <div className={styles.priceMain}>
          <span className={styles.priceAmount}>{agent.price.toFixed(2)}</span>
          <span className={styles.priceUnit}>XLM</span>
        </div>
        <span className={styles.priceUsd}>≈ ${usdEstimate}</span>
      </div>

      <div className={styles.reputationRow}>
        <ReputationStars value={agent.reputation} />
      </div>

      {/* ── Hire CTA — hidden by default, visible on card hover ──── */}
      <button
        type="button"
        className={styles.hireCta}
        onClick={handleHire}
        disabled={!isOnline}
        aria-disabled={!isOnline}
        data-testid={`agent-hire-${agent.id}`}
        aria-label={`Hire ${agent.name}`}
        tabIndex={0}
      >
        {isOnline
          ? t('agent.hire', { defaultValue: 'Hire Agent' })
          : t('agent.offline', { defaultValue: 'Offline' })}
      </button>
    </article>
  );
};

export default AgentRegistryCard;
