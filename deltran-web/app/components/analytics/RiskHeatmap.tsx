'use client'

import { useRef, useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import * as d3 from 'd3'
import { RiskCell } from '../../hooks/useRiskData'
import { AlertTriangle } from 'lucide-react'

interface RiskHeatmapProps {
  cells: RiskCell[]
  maxScore: number
  onCellClick?: (cell: RiskCell) => void
}

const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
const CELL_SIZE = 20
const CELL_PADDING = 2

export function RiskHeatmap({ cells, maxScore, onCellClick }: RiskHeatmapProps) {
  const svgRef = useRef<SVGSVGElement>(null)
  const [hoveredCell, setHoveredCell] = useState<RiskCell | null>(null)
  const [tooltipPos, setTooltipPos] = useState({ x: 0, y: 0 })

  // Color scale: green -> yellow -> orange -> red
  const colorScale = d3
    .scaleLinear<string>()
    .domain([0, 25, 50, 75, 100])
    .range(['#10b981', '#84cc16', '#eab308', '#f97316', '#ef4444'])

  useEffect(() => {
    if (!svgRef.current || cells.length === 0) return

    const svg = d3.select(svgRef.current)
    svg.selectAll('*').remove()

    const width = 24 * (CELL_SIZE + CELL_PADDING)
    const height = 7 * (CELL_SIZE + CELL_PADDING)

    // Add cells
    const cellsGroup = svg
      .append('g')
      .attr('transform', `translate(60, 20)`)

    cells.forEach((cell, index) => {
      const x = cell.hour * (CELL_SIZE + CELL_PADDING)
      const y = cell.day * (CELL_SIZE + CELL_PADDING)
      const isCritical = cell.score > 75

      const rect = cellsGroup
        .append('rect')
        .attr('x', x)
        .attr('y', y)
        .attr('width', 0)
        .attr('height', 0)
        .attr('rx', 3)
        .attr('fill', colorScale(cell.score))
        .attr('stroke', isCritical ? '#d4af37' : 'none')
        .attr('stroke-width', isCritical ? 2 : 0)
        .style('cursor', 'pointer')
        .style('opacity', 0)

      // Wave animation: cells appear from left to right
      rect
        .transition()
        .delay(index * 3)
        .duration(400)
        .attr('width', CELL_SIZE)
        .attr('height', CELL_SIZE)
        .style('opacity', 1)

      // Pulsation for critical cells
      if (isCritical) {
        const pulse = () => {
          rect
            .transition()
            .duration(1000)
            .attr('stroke-width', 3)
            .attr('stroke-opacity', 1)
            .transition()
            .duration(1000)
            .attr('stroke-width', 1)
            .attr('stroke-opacity', 0.5)
            .on('end', pulse)
        }
        setTimeout(() => pulse(), index * 3 + 400)
      }

      // Hover effects
      rect
        .on('mouseenter', function (event) {
          d3.select(this)
            .transition()
            .duration(150)
            .attr('width', CELL_SIZE + 4)
            .attr('height', CELL_SIZE + 4)
            .attr('x', x - 2)
            .attr('y', y - 2)

          const svgRect = svgRef.current!.getBoundingClientRect()
          setTooltipPos({
            x: event.clientX - svgRect.left,
            y: event.clientY - svgRect.top,
          })
          setHoveredCell(cell)
        })
        .on('mouseleave', function () {
          d3.select(this)
            .transition()
            .duration(150)
            .attr('width', CELL_SIZE)
            .attr('height', CELL_SIZE)
            .attr('x', x)
            .attr('y', y)

          setHoveredCell(null)
        })
        .on('click', () => {
          if (onCellClick) onCellClick(cell)
        })
    })

    // Add day labels
    DAYS.forEach((day, index) => {
      svg
        .append('text')
        .attr('x', 50)
        .attr('y', 20 + index * (CELL_SIZE + CELL_PADDING) + CELL_SIZE / 2)
        .attr('text-anchor', 'end')
        .attr('dominant-baseline', 'middle')
        .attr('fill', '#9ca3af')
        .attr('font-size', '12px')
        .text(day)
    })

    // Add hour labels (every 6 hours)
    for (let hour = 0; hour < 24; hour += 6) {
      svg
        .append('text')
        .attr('x', 60 + hour * (CELL_SIZE + CELL_PADDING))
        .attr('y', 12)
        .attr('text-anchor', 'middle')
        .attr('fill', '#9ca3af')
        .attr('font-size', '11px')
        .text(`${hour}:00`)
    }
  }, [cells, colorScale, onCellClick])

  const formatCurrency = (value: number) =>
    new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', notation: 'compact' }).format(value)

  return (
    <div className="rounded-xl p-6 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-white">Risk Score Distribution</h3>
          <p className="text-sm text-zinc-400">24h x 7d heatmap</p>
        </div>
        <div className="flex items-center gap-2 text-xs text-zinc-400">
          <div className="flex items-center gap-1">
            <div className="w-3 h-3 rounded bg-gradient-to-r from-[#10b981] to-[#ef4444]" />
            <span>0-100 Risk Score</span>
          </div>
        </div>
      </div>

      <div className="relative">
        <svg
          ref={svgRef}
          className="w-full overflow-visible"
          style={{ height: '200px' }}
        />

        {/* Tooltip */}
        {hoveredCell && (
          <motion.div
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            className="absolute z-50 pointer-events-none"
            style={{
              left: tooltipPos.x + 10,
              top: tooltipPos.y - 60,
            }}
          >
            <div className="bg-black border border-deltran-gold/30 rounded-lg p-3 shadow-xl">
              <div className="text-xs text-zinc-400 mb-1">
                {DAYS[hoveredCell.day]} {hoveredCell.hour}:00
              </div>
              <div className="text-sm font-semibold text-white mb-1">
                Risk Score: <span className="text-deltran-gold">{hoveredCell.score}</span>
              </div>
              <div className="text-xs text-zinc-400">
                {hoveredCell.transactionCount} transactions
              </div>
              <div className="text-xs text-zinc-400">
                Volume: {formatCurrency(hoveredCell.totalVolume)}
              </div>
              {hoveredCell.score > 75 && (
                <div className="flex items-center gap-1 mt-2 text-xs text-red-400">
                  <AlertTriangle className="w-3 h-3" />
                  <span>Critical Risk</span>
                </div>
              )}
            </div>
          </motion.div>
        )}
      </div>

      {/* Legend */}
      <div className="mt-4 flex items-center justify-between text-xs text-zinc-500">
        <span>Hover cells for details • Click to filter</span>
        <span>Gold border = Critical risk (&gt;75)</span>
      </div>
    </div>
  )
}
