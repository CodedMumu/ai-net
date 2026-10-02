import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronDown, ChevronUp, ExternalLink, Zap } from 'lucide-react'
import type { AgentRecord } from '../../types/api'
import { ReputationStars } from './ReputationStars'
import { useAgentReputation } from '../../hooks/useAgentReputation'
import { AgentReputationRadar } from './AgentReputationRadar'
import { AgentReputationTrend } from './AgentReputationTrend'
import { SkeletonText } from '../common/Skeleton'
import styles from './AgentDetailModal.module.css'

const STELLAR_EXPLORER = 'https://stellar.expert/explorer/testnet'

// Tab identifiers for URL hash routing
type TabId = 'history' | 'reviews' | 'metrics'
const TABS: { id: TabId; labelKey: string }[] = [
  { id: 'history', labelKey: 'agent.modal.tab.history' },
  { id: 'reviews', labelKey: 'agent.modal.tab.reviews' },
  { id: 'metrics', labelKey: 'agent.modal.tab.metrics' },
]

function getHashTab(): TabId {
  const hash = window.location.hash.slice(1)
  if (hash === 'history' || hash === 'reviews' || hash === 'metrics') return hash
  return 'history'
}

interface AgentDetailModalProps {
  agent: AgentRecord | null
  onClose: () => void
}

export function AgentDetailModal({ agent, onClose }: AgentDetailModalProps) {
  const { t } = useTranslation()
  const { data: reputationData, loading: reputationLoading } = useAgentReputation(agent?.id ?? '')

  const [expanded, setExpanded] = useState(false)
  const [activeTab, setActiveTab] = useState<TabId>(getHashTab)
  const expandableRef = useRef<HTMLDivElement>(null)
  const closeButtonRef = useRef<HTMLButtonElement>(null)
  const dialogRef = useRef<HTMLDivElement>(null)

  // Sync tab with URL hash
  useEffect(() => {
    const onHashChange = () => setActiveTab(getHashTab())
    window.addEventListener('hashchange', onHashChange)
    return () => window.removeEventListener('hashchange', onHashChange)
  }, [])

  // Keyboard + focus-trap handler
  useEffect(() => {
    if (!agent) return

    // Focus the close button when the modal opens
    setTimeout(() => closeButtonRef.current?.focus(), 50)

    const focusableSelectors =
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
        return
      }
      if (e.key === 'Tab' && dialogRef.current) {
        const focusable = Array.from(
          dialogRef.current.querySelectorAll<HTMLElement>(focusableSelectors)
        ).filter((el) => !el.hasAttribute('disabled'))
        if (focusable.length === 0) return
        const first = focusable[0]
        const last = focusable[focusable.length - 1]
        if (e.shiftKey) {
          if (document.activeElement === first) {
            e.preventDefault()
            last.focus()
          }
        } else {
          if (document.activeElement === last) {
            e.preventDefault()
            first.focus()
          }
        }
      }
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [agent, onClose])

  // Reset expanded state when agent changes
  useEffect(() => {
    setExpanded(false)
  }, [agent?.id])

  const handleTabClick = (id: TabId) => {
    setActiveTab(id)
    window.history.replaceState(null, '', `#${id}`)
  }

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) onClose()
  }

  if (!agent) return null

  const topCapabilities = agent.capabilities.slice(0, 3)
  const extraCapabilities = agent.capabilities.slice(3)
  const isActive = agent.status === 'active'

  return (
    <div
      className={styles.backdrop}
      role="presentation"
      onClick={handleBackdropClick}
      aria-hidden="false"
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="agent-modal-title"
        className={styles.modal}
        data-testid="agent-detail-modal"
      >
        {/* ── Close button ── */}
        <button
          ref={closeButtonRef}
          type="button"
          className={styles.closeButton}
          aria-label={t('common.close')}
          onClick={onClose}
        >
          ✕
        </button>

        {/* ════════════════════════════════
            PRIMARY SECTION — always visible
        ════════════════════════════════ */}
        <section className={styles.primarySection} aria-label={t('agent.modal.primarySectionLabel')}>
          <div className={styles.primaryHeader}>
            <div className={styles.agentTitleRow}>
              <h2 id="agent-modal-title" className={styles.agentName}>
                {agent.name}
              </h2>
              <span
                className={`${styles.statusBadge} ${isActive ? styles.statusActive : styles.statusInactive}`}
                aria-label={`${t('common.status')}: ${t(`agent.status.${agent.status}`, { defaultValue: agent.status })}`}
              >
                <span className={styles.statusDot} aria-hidden="true" />
                {t(`agent.status.${agent.status}`, { defaultValue: agent.status })}
              </span>
            </div>

            <div className={styles.primaryMeta}>
              <div className={styles.priceBlock}>
                <span className={styles.priceLabel}>{t('common.price')}</span>
                <span className={styles.priceValue}>{agent.price.toFixed(2)} XLM</span>
              </div>
              <div className={styles.reputationBlock}>
                <span className={styles.reputationLabel}>{t('common.reputation')}</span>
                <ReputationStars value={agent.reputation} />
              </div>
            </div>
          </div>

          {/* Top 3 capabilities */}
          <div className={styles.capabilitiesRow}>
            <span className={styles.capsLabel}>{t('agent.modal.topCapabilities')}</span>
            <div className={styles.pillRow} role="list" aria-label={t('common.capabilities')}>
              {topCapabilities.length === 0 ? (
                <span className={styles.muted}>—</span>
              ) : (
                topCapabilities.map((cap) => (
                  <span key={cap} className={styles.pill} role="listitem">
                    {cap}
                  </span>
                ))
              )}
              {extraCapabilities.length > 0 && !expanded && (
                <span className={styles.pillMore} aria-hidden="true">
                  +{extraCapabilities.length}
                </span>
              )}
            </div>
          </div>

          {/* Hire button */}
          <button
            type="button"
            className={styles.hireButton}
            disabled={!isActive}
            aria-disabled={!isActive}
            aria-label={
              isActive
                ? t('agent.modal.hire', { name: agent.name })
                : t('agent.modal.hireUnavailable', { name: agent.name })
            }
          >
            <Zap size={16} aria-hidden="true" />
            {t('agent.modal.hire', { name: agent.name })}
          </button>
        </section>

        {/* ═══════════════════════════════════════
            SECONDARY SECTION — expandable details
        ═══════════════════════════════════════ */}
        <section className={styles.secondarySection}>
          <button
            type="button"
            className={styles.toggleButton}
            aria-expanded={expanded}
            aria-controls="agent-secondary-content"
            onClick={() => setExpanded((v) => !v)}
          >
            {expanded ? (
              <>
                <ChevronUp size={16} aria-hidden="true" />
                {t('agent.modal.showLess')}
              </>
            ) : (
              <>
                <ChevronDown size={16} aria-hidden="true" />
                {t('agent.modal.showMore')}
              </>
            )}
          </button>

          {/* Animated expandable panel */}
          <div
            id="agent-secondary-content"
            ref={expandableRef}
            className={`${styles.expandableContent} ${expanded ? styles.expandableOpen : ''}`}
            // Keep in DOM but hide from AT when collapsed — aria-hidden reflects visual state
            aria-hidden={!expanded}
            // inert when collapsed so keyboard can't reach hidden content
            {...(!expanded ? { inert: '' } : {})}
          >
            <dl className={styles.detailGrid}>
              {/* Full capability list */}
              <div className={styles.detailRow}>
                <dt>{t('common.capabilities')}</dt>
                <dd>
                  <div className={styles.pillRow} role="list">
                    {agent.capabilities.length === 0 ? (
                      <span className={styles.muted}>—</span>
                    ) : (
                      agent.capabilities.map((cap) => (
                        <span key={cap} className={styles.pill} role="listitem">
                          {cap}
                        </span>
                      ))
                    )}
                  </div>
                </dd>
              </div>

              {/* Endpoint */}
              {agent.endpoint && (
                <div className={styles.detailRow}>
                  <dt>{t('agent.modal.endpoint')}</dt>
                  <dd className={styles.mono}>{agent.endpoint}</dd>
                </div>
              )}

              {/* Registration TX */}
              <div className={styles.detailRow}>
                <dt>{t('agent.modal.registrationTx')}</dt>
                <dd>
                  {agent.registrationTxHash ? (
                    <a
                      className={styles.txLink}
                      href={`${STELLAR_EXPLORER}/tx/${agent.registrationTxHash}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      data-testid="registration-tx-link"
                    >
                      <span className={styles.mono}>{agent.registrationTxHash}</span>
                      <ExternalLink size={14} aria-hidden="true" />
                    </a>
                  ) : (
                    <span className={styles.muted}>{t('common.notAvailable')}</span>
                  )}
                </dd>
              </div>

              {/* Agent ID */}
              <div className={styles.detailRow}>
                <dt>{t('agent.modal.agentId')}</dt>
                <dd>
                  <code className={styles.mono}>{agent.id}</code>
                </dd>
              </div>
            </dl>
          </div>
        </section>

        {/* ══════════════════════════════
            THIRD SECTION — tabbed content
        ══════════════════════════════ */}
        <section className={styles.tabSection} aria-label={t('agent.modal.detailsLabel')}>
          {/* Tab bar */}
          <div role="tablist" className={styles.tabList} aria-label={t('agent.modal.tabsLabel')}>
            {TABS.map(({ id, labelKey }) => (
              <button
                key={id}
                type="button"
                role="tab"
                id={`tab-${id}`}
                aria-selected={activeTab === id}
                aria-controls={`tabpanel-${id}`}
                className={`${styles.tab} ${activeTab === id ? styles.tabActive : ''}`}
                onClick={() => handleTabClick(id)}
                tabIndex={activeTab === id ? 0 : -1}
                onKeyDown={(e) => {
                  const ids = TABS.map((t) => t.id)
                  const idx = ids.indexOf(id)
                  if (e.key === 'ArrowRight') {
                    handleTabClick(ids[(idx + 1) % ids.length])
                  } else if (e.key === 'ArrowLeft') {
                    handleTabClick(ids[(idx - 1 + ids.length) % ids.length])
                  }
                }}
              >
                {t(labelKey)}
              </button>
            ))}
          </div>

          {/* Tab panels */}
          <div className={styles.tabPanelContainer}>
            {/* History panel */}
            <div
              role="tabpanel"
              id="tabpanel-history"
              aria-labelledby="tab-history"
              className={`${styles.tabPanel} ${activeTab === 'history' ? styles.tabPanelActive : ''}`}
              tabIndex={0}
              hidden={activeTab !== 'history'}
            >
              <div className={styles.tabPanelContent}>
                <p className={styles.tabPlaceholder}>{t('agent.modal.historyPlaceholder')}</p>
              </div>
            </div>

            {/* Reviews panel */}
            <div
              role="tabpanel"
              id="tabpanel-reviews"
              aria-labelledby="tab-reviews"
              className={`${styles.tabPanel} ${activeTab === 'reviews' ? styles.tabPanelActive : ''}`}
              tabIndex={0}
              hidden={activeTab !== 'reviews'}
            >
              <div className={styles.tabPanelContent}>
                {reputationLoading ? (
                  <SkeletonText lines={4} />
                ) : reputationData ? (
                  <div className={styles.chartsContainer}>
                    <AgentReputationRadar dimensions={reputationData.dimensions} />
                    <AgentReputationTrend history={reputationData.history} />
                  </div>
                ) : (
                  <p className={styles.tabPlaceholder}>{t('agent.modal.reviewsPlaceholder')}</p>
                )}
              </div>
            </div>

            {/* Metrics panel */}
            <div
              role="tabpanel"
              id="tabpanel-metrics"
              aria-labelledby="tab-metrics"
              className={`${styles.tabPanel} ${activeTab === 'metrics' ? styles.tabPanelActive : ''}`}
              tabIndex={0}
              hidden={activeTab !== 'metrics'}
            >
              <div className={styles.tabPanelContent}>
                <p className={styles.tabPlaceholder}>{t('agent.modal.metricsPlaceholder')}</p>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  )
}
