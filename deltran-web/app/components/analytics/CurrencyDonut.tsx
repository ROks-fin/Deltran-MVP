'use client'

import { useEffect, useRef, useState } from 'react'
import { motion } from 'framer-motion'
import * as d3 from 'd3'
import { CurrencyData } from '../../hooks/useCurrencyDistribution'
import { TrendingUp, TrendingDown, Minus } from 'lucide-react'

interface CurrencyDonutProps {
  currencies: CurrencyData[]
  totalVolume: number
  dominantCurrency: string
}

const CURRENCY_COLORS: Record<string, string> = {
  USD: '#4ade80',
  EUR: '#60a5fa',
  GBP: '#a78bfa',
  AED: '#fbbf24',
  INR: '#fb923c',
  PKR: '#f87171',
  ILS: '#38bdf8',
}

export function CurrencyDonut({ currencies, totalVolume, dominantCurrency }: CurrencyDonutProps) {
  const svgRef = useRef<SVGSVGElement>(null)
  const [hoveredCurrency, setHoveredCurrency] = useState<CurrencyData | null>(null)

  useEffect(() => {
    if (!svgRef.current || currencies.length === 0) return

    const svg = d3.select(svgRef.current)
    svg.selectAll('*').remove()

    const width = 280
    const height = 280
    const radius = Math.min(width, height) / 2 - 10
    const innerRadius = radius * 0.6

    const g = svg
      .append('g')
      .attr('transform', `translate(${width / 2}, ${height / 2})`)

    const pie = d3.pie<CurrencyData>().value((d) => d.value)

    const arc = d3
      .arc<d3.PieArcDatum<CurrencyData>>()
      .innerRadius(innerRadius)
      .outerRadius(radius)

    const arcHover = d3
      .arc<d3.PieArcDatum<CurrencyData>>()
      .innerRadius(innerRadius)
      .outerRadius(radius + 10)

    const arcs = g
      .selectAll('.arc')
      .data(pie(currencies))
      .enter()
      .append('g')
      .attr('class', 'arc')

    // Animate arcs appearing clockwise
    arcs
      .append('path')
      .attr('fill', (d) => CURRENCY_COLORS[d.data.currency] || '#666')
      .attr('stroke', '#0a0a0a')
      .attr('stroke-width', 2)
      .style('cursor', 'pointer')
      .style('opacity', 0)
      .transition()
      .delay((d, i) => i * 100)
      .duration(600)
      .style('opacity', 1)
      .attrTween('d', function (d) {
        const interpolate = d3.interpolate({ startAngle: 0, endAngle: 0 }, d)
        return function (t) {
          return arc(interpolate(t)) || ''
        }
      })

    // Add glow for dominant currency
    arcs
      .filter((d) => d.data.currency === dominantCurrency)
      .append('path')
      .attr('fill', 'none')
      .attr('stroke', '#d4af37')
      .attr('stroke-width', 3)
      .attr('d', (d) => arc(d) || '')
      .style('filter', 'drop-shadow(0 0 8px #d4af37)')
      .style('opacity', 0)
      .transition()
      .delay(700)
      .duration(400)
      .style('opacity', 0.8)

    // Hover interactions
    arcs
      .selectAll('path')
      .on('mouseenter', function (event, d: any) {
        d3.select(this)
          .transition()
          .duration(200)
          .attr('d', arcHover(d) || '')

        setHoveredCurrency(d.data)
      })
      .on('mouseleave', function (event, d: any) {
        d3.select(this)
          .transition()
          .duration(200)
          .attr('d', arc(d) || '')

        setHoveredCurrency(null)
      })

    // Center text - total volume
    const centerText = svg
      .append('g')
      .attr('transform', `translate(${width / 2}, ${height / 2})`)

    centerText
      .append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', '-0.5em')
      .attr('fill', '#9ca3af')
      .attr('font-size', '12px')
      .text('Total Volume')

    centerText
      .append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', '0.8em')
      .attr('fill', '#ffffff')
      .attr('font-size', '20px')
      .attr('font-weight', 'bold')
      .style('opacity', 0)
      .text(formatCurrency(totalVolume))
      .transition()
      .delay(800)
      .duration(400)
      .style('opacity', 1)
  }, [currencies, totalVolume, dominantCurrency])

  const formatCurrency = (value: number) =>
    new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      notation: 'compact',
      maximumFractionDigits: 1,
    }).format(value)

  const getTrendIcon = (trend: string) => {
    switch (trend) {
      case 'up':
        return <TrendingUp className="w-3 h-3 text-green-400" />
      case 'down':
        return <TrendingDown className="w-3 h-3 text-red-400" />
      default:
        return <Minus className="w-3 h-3 text-zinc-400" />
    }
  }

  return (
    <div className="rounded-xl p-6 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-white">Currency Distribution</h3>
          <p className="text-sm text-zinc-400">Volume by currency</p>
        </div>
      </div>

      <div className="flex items-center gap-6">
        {/* Donut Chart */}
        <div className="relative">
          <svg ref={svgRef} width="280" height="280" />

          {/* Hover info overlay */}
          {hoveredCurrency && (
            <motion.div
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 pointer-events-none"
            >
              <div className="text-center">
                <div className="text-2xl font-bold text-white mb-1">{hoveredCurrency.currency}</div>
                <div className="text-sm text-deltran-gold">{hoveredCurrency.percentage.toFixed(1)}%</div>
                <div className="text-xs text-zinc-400">{formatCurrency(hoveredCurrency.value)}</div>
              </div>
            </motion.div>
          )}
        </div>

        {/* Legend */}
        <div className="flex-1 space-y-2">
          {currencies.map((currency, index) => (
            <motion.div
              key={currency.currency}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.1 + 0.3, duration: 0.4 }}
              className="flex items-center justify-between group hover:bg-zinc-800/30 rounded-lg p-2 cursor-pointer transition-colors"
            >
              <div className="flex items-center gap-2">
                <div
                  className="w-3 h-3 rounded-full"
                  style={{ backgroundColor: CURRENCY_COLORS[currency.currency] || '#666' }}
                />
                <span className="text-sm font-medium text-white">{currency.currency}</span>
                {currency.currency === dominantCurrency && (
                  <span className="text-xs text-deltran-gold">★</span>
                )}
              </div>

              <div className="flex items-center gap-3">
                <div className="text-right">
                  <div className="text-sm text-white">{currency.percentage.toFixed(1)}%</div>
                  <div className="text-xs text-zinc-400">{currency.count} txns</div>
                </div>
                {getTrendIcon(currency.trend)}
              </div>
            </motion.div>
          ))}
        </div>
      </div>

      {/* Footer */}
      <div className="mt-4 pt-4 border-t border-zinc-800 text-xs text-zinc-500">
        Hover chart segments for details • Gold glow = dominant currency
      </div>
    </div>
  )
}
